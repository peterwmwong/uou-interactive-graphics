use crate::{
    objc_helpers::debug_assert_objc_class,
    renderer::{MetalRenderer, RendererDelgate},
    unwrap_helpers::unwrap_option_dcheck,
    ModifierKeys, MouseButton, UserEvent,
};
use cocoa::{
    appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicy,
        NSBackingStoreType::NSBackingStoreBuffered, NSEvent, NSEventModifierFlags, NSEventType,
        NSMenu, NSMenuItem, NSWindow, NSWindowStyleMask,
    },
    base::{id, nil, selector},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString},
};
use dispatch::Queue;
use display_link::DisplayLink;
use objc::{
    declare::ClassDecl,
    rc::autoreleasepool,
    runtime::{Object, Sel, BOOL, YES},
};
use std::{os::raw::c_void, simd::f32x2};

#[inline]
fn parse_modifier_keys(ns_modifiers: NSEventModifierFlags) -> ModifierKeys {
    let mut modifiers = ModifierKeys::empty();
    for (ns_modifier, modifier) in [
        (NSEventModifierFlags::NSShiftKeyMask, ModifierKeys::SHIFT),
        (
            NSEventModifierFlags::NSControlKeyMask,
            ModifierKeys::CONTROL,
        ),
        (
            NSEventModifierFlags::NSCommandKeyMask,
            ModifierKeys::COMMAND,
        ),
        (
            NSEventModifierFlags::NSFunctionKeyMask,
            ModifierKeys::FUNCTION,
        ),
    ] {
        if ns_modifiers.contains(ns_modifier) {
            modifiers |= modifier;
        };
    }
    modifiers
}

// Important: Call within `autoreleasepool()`.
pub fn from_nswindow<R: RendererDelgate + 'static>(nswindow: *mut Object) -> DisplayLink {
    let nswindow = debug_assert_objc_class(nswindow, &"NSWindow");
    let backing_scale_factor = unsafe { nswindow.backingScaleFactor() as f32 };
    let mut renderer: Box<MetalRenderer<R>> = Box::new(MetalRenderer::new(backing_scale_factor));
    init_window_event_handlers::<R>(nswindow, &mut renderer);
    init_and_attach_view::<R>(nswindow, &mut renderer);

    let main_queue = Queue::main();
    DisplayLink::new(move |_| {
        if renderer.needs_render() {
            main_queue.exec_sync(|| renderer.render());
        }
    })
    .expect("Could not create Display Link")
}

const RENDERER_IVAR: &'static str = "renderer";

#[inline(always)]
fn get_renderer<R: RendererDelgate + 'static>(this: &Object) -> &mut MetalRenderer<R> {
    unsafe { &mut *(*this.get_ivar::<*mut c_void>(RENDERER_IVAR) as *mut MetalRenderer<R>) }
}

fn init_and_attach_view<R: RendererDelgate + 'static>(
    nswindow: *mut Object,
    renderer: &mut Box<MetalRenderer<R>>,
) {
    unsafe {
        use cocoa::appkit::NSView;
        let mut decl = unwrap_option_dcheck(
            ClassDecl::new("CustomNSView", class!(NSView)),
            "Unable to create custom NSView (CustomNSView)",
        );
        decl.add_method(sel!(acceptsFirstResponder), {
            extern "C" fn accepts_first_responder(_this: &Object, _sel: Sel) -> BOOL {
                YES
            }
            accepts_first_responder as extern "C" fn(&Object, Sel) -> BOOL
        });
        for selector in [
            sel!(mouseDown:),
            sel!(mouseDragged:),
            sel!(mouseUp:),
            sel!(rightMouseDown:),
            sel!(rightMouseDragged:),
            sel!(rightMouseUp:),
        ] {
            decl.add_method(selector, {
                extern "C" fn on_mouse_event<R: RendererDelgate + 'static>(
                    this: &mut Object,
                    _: Sel,
                    event: *mut Object,
                ) {
                    use MouseButton::*;
                    use NSEventType::*;
                    use UserEvent::*;
                    static mut LAST_DRAG_POSITION: f32x2 = f32x2::splat(0.0);

                    // We have to do this to have access to the `NSView` trait...
                    let view: id = this;
                    let position = unsafe {
                        let view_rect = NSView::frame(view);
                        let NSPoint { x, y } =
                            view.convertPoint_fromView_(event.locationInWindow(), nil);
                        if x < 0.0
                            || y < 0.0
                            || x > view_rect.size.width
                            || y > view_rect.size.height
                        {
                            return;
                        }

                        let point = view
                            .convertRectToBacking(NSRect::new(
                                NSPoint {
                                    x,
                                    /*

                                    IMPORTANT: Flips y coordinate to match application coordinate system...

                                    BEFORE: OS coordinate system
                                    ========================================================================

                                        (0,height)
                                        ^
                                        |
                                        (0,0) -> (width, 0)

                                    AFTER: Application coordinate system. Also matches Metal Viewport
                                           Coordinate system, see "Metal Coordinate System"
                                           https://developer.apple.com/metal/Metal-Shading-Language-Specification.pdf
                                    ========================================================================

                                        (0,0) -> (width, 0)
                                        |
                                        v
                                        (0,height)

                                    */
                                    y: view_rect.size.height - y,
                                },
                                NSSize::new(0.0, 0.0),
                            ))
                            .origin;
                        f32x2::from_array([point.x as f32, point.y as f32])
                    };
                    let ns_event_type = unsafe { NSEvent::eventType(event) };
                    let modifier_keys =
                        parse_modifier_keys(unsafe { NSEvent::modifierFlags(event) });

                    let button = match ns_event_type {
                        NSLeftMouseDown | NSLeftMouseDragged | NSLeftMouseUp => Left,
                        NSRightMouseDown | NSRightMouseDragged | NSRightMouseUp => Right,
                        unknown_nseventtype @ _ => {
                            dbg!(unknown_nseventtype);
                            return;
                        }
                    };
                    let user_event = match ns_event_type {
                        NSLeftMouseDown | NSRightMouseDown => {
                            unsafe { LAST_DRAG_POSITION = position };
                            MouseDown {
                                button,
                                position,
                                modifier_keys,
                            }
                        }
                        NSLeftMouseDragged | NSRightMouseDragged => {
                            let drag_amount = unsafe { LAST_DRAG_POSITION - position };
                            unsafe { LAST_DRAG_POSITION = position };
                            MouseDrag {
                                button,
                                modifier_keys,
                                position,
                                drag_amount,
                            }
                        }
                        NSLeftMouseUp | NSRightMouseUp => MouseUp {
                            button,
                            modifier_keys,
                            position,
                        },
                        _ => return, // Should never get here, preceding match should have early exited
                                     // in the case.
                    };
                    get_renderer::<R>(this).on_event(user_event);
                }
                on_mouse_event::<R> as extern "C" fn(&mut Object, Sel, id)
            });
        }
        decl.add_method(sel!(keyDown:), {
            extern "C" fn on_key_down<R: RendererDelgate + 'static>(
                this: &Object,
                _: Sel,
                event: *mut Object,
            ) {
                unsafe {
                    let key_code = NSEvent::keyCode(event);
                    const ESCAPE_KEY: u16 = 53;
                    if key_code == ESCAPE_KEY {
                        let () = msg_send![NSApp(), terminate: nil];
                    } else {
                        get_renderer::<R>(this).on_event(UserEvent::KeyDown { key_code });
                    }
                }
            }
            on_key_down::<R> as extern "C" fn(&Object, Sel, id)
        });
        decl.add_ivar::<*mut c_void>(RENDERER_IVAR);
        let viewclass = decl.register();
        let view: id = msg_send![viewclass, alloc];
        let () = msg_send![view, init];
        let renderer_ptr: *mut MetalRenderer<R> = &mut **renderer;
        (&mut *view).set_ivar::<*mut c_void>(RENDERER_IVAR, renderer_ptr as *mut c_void);
        view.setWantsLayer(YES);
        view.setLayer(std::mem::transmute(renderer.layer.as_ref()));
        nswindow.setContentView_(view);
        nswindow.setInitialFirstResponder_(view);
    }
}

fn init_window_event_handlers<R: RendererDelgate + 'static>(
    nswindow: *mut Object,
    renderer: &mut Box<MetalRenderer<R>>,
) {
    let renderer_ptr: *mut MetalRenderer<R> = &mut **renderer;

    extern "C" fn on_nswindow_resize<R: RendererDelgate + 'static>(
        this: &Object,
        _: Sel,
        notification: *mut Object,
    ) {
        let NSSize { width, height } = unsafe {
            // "Discussion: You can retrieve the window object in question by sending object to notification."
            //   - https://developer.apple.com/documentation/appkit/nswindowdelegate/1419567-windowdidresize?language=objc
            //   - https://developer.apple.com/documentation/appkit/nswindowdelegate/1419190-windowdidbecomemain?language=objc
            let nswindow: *mut Object = msg_send![
                debug_assert_objc_class(notification, &"NSNotification"),
                object
            ];
            nswindow.contentRectForFrameRect_(nswindow.frame()).size
        };
        get_renderer::<R>(this).update_size(f32x2::from_array([width as f32, height as f32]));
    }

    unsafe {
        #[allow(non_camel_case_types)]
        type id = cocoa::base::id; // Used by code generated by `delegate!` macro.
        debug_assert_objc_class(nswindow, &"NSWindow").setDelegate_(delegate!("WindowDelegate", {
                // Instance Variables (Retrieved using `this.get_ivar()`)
                renderer: *mut c_void = renderer_ptr as *mut c_void,
                // Callback Functions
                (windowDidResize:) => on_nswindow_resize::<R> as extern fn(&Object, Sel, *mut Object),
                (windowDidBecomeMain:) => on_nswindow_resize::<R> as extern fn(&Object, Sel, *mut Object)
            }));
    }
}

pub fn launch_application<R: RendererDelgate + 'static>(app_name: &'static str) {
    autoreleasepool(|| unsafe {
        let app = NSApp();
        app.setActivationPolicy_(
            NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular,
        );
        app.setMainMenu_({
            // Create Menu Bar
            let menubar = NSMenu::new(nil).autorelease();

            // Create Application menu
            menubar.addItem_({
                let menu_item = NSMenuItem::new(nil).autorelease();
                let menu = NSMenu::new(nil).autorelease();
                menu.addItem_(
                    NSMenuItem::alloc(nil)
                        .initWithTitle_action_keyEquivalent_(
                            NSString::alloc(nil)
                                .init_str(&format!("Quit {app_name}"))
                                .autorelease(),
                            selector("terminate:"),
                            NSString::alloc(nil).init_str("q").autorelease(),
                        )
                        .autorelease(),
                );
                menu_item.setSubmenu_(menu);
                menu_item
            });
            menubar
        });
        let mut link = from_nswindow::<R>({
            let window = NSWindow::alloc(nil)
                .initWithContentRect_styleMask_backing_defer_(
                    NSRect::new(NSPoint::new(0., 0.), NSSize::new(512.0, 512.0)),
                    // TODO: Consider rendering a custom title or no title bar at all
                    //       To maintain resizability...
                    //       - use NSWindowStyleMask::NSResizableWindowMask | NSWindowStyleMask::NSFullSizeContentViewWindowMask,
                    //       - window.canBecomeKeyWindow();
                    //       - window.canBecomeMainWindow();
                    NSWindowStyleMask::NSClosableWindowMask
                        | NSWindowStyleMask::NSTitledWindowMask
                        | NSWindowStyleMask::NSResizableWindowMask,
                    NSBackingStoreBuffered,
                    YES,
                )
                .autorelease();
            window.setAcceptsMouseMovedEvents_(YES);
            window.setPreservesContentDuringLiveResize_(false);
            window.setTitle_(NSString::alloc(nil).init_str(app_name).autorelease());
            window.makeKeyAndOrderFront_(nil);
            window
        });
        app.activateIgnoringOtherApps_(true);
        link.resume().expect("Could not start Display Link");
        app.run();
    });
}
