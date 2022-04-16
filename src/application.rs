use crate::{
    objc_helpers::debug_assert_objc_class,
    renderer::{MetalRenderer, Size, Unit},
    unwrap_helpers::unwrap_option_dcheck,
};
use cocoa::{
    appkit::{
        NSApp, NSApplication, NSApplicationActivationPolicy,
        NSBackingStoreType::NSBackingStoreBuffered, NSEvent, NSMenu, NSMenuItem, NSWindow,
        NSWindowStyleMask,
    },
    base::{id, nil, selector},
    foundation::{NSAutoreleasePool, NSPoint, NSRect, NSSize, NSString},
};
use objc::{
    declare::ClassDecl,
    rc::autoreleasepool,
    runtime::{Object, Sel, BOOL, YES},
};
use std::ffi::c_void;

const APP_NAME: &str = "UOU Interactive Graphics";

pub struct ApplicationManager {
    renderer: MetalRenderer,
}

impl ApplicationManager {
    // Important: Call within `autoreleasepool()`.
    pub fn from_nswindow(nswindow: *mut Object) -> Box<Self> {
        let nswindow = debug_assert_objc_class(nswindow, &"NSWindow");
        let mut manager = Box::new(Self {
            renderer: MetalRenderer::new(unsafe { nswindow.backingScaleFactor() as Unit }),
        });
        manager.init_window_event_handlers(nswindow);
        manager.init_and_attach_view(nswindow);
        manager
    }

    fn init_and_attach_view(self: &mut Box<Self>, nswindow: *mut Object) {
        use cocoa::appkit::NSView;
        unsafe {
            let superclass = class!(NSView);
            let mut decl = unwrap_option_dcheck(
                ClassDecl::new("CustomNSView", superclass),
                "Unable to create custom NSView (CustomNSView)",
            );

            extern "C" fn accepts_first_responder(_this: &Object, _sel: Sel) -> BOOL {
                YES
            }

            extern "C" fn on_mouse_moved(this: &Object, _: Sel, event: *mut Object) {
                debug_assert_objc_class(event, "NSEvent");
                // We have to do this to have access to the `NSView` trait...
                let view: id = this as *const _ as *mut _;
                let view_rect = unsafe { NSView::frame(view) };
                let NSPoint { x: _x, y: _y } = unsafe {
                    let NSPoint { x, y } =
                        view.convertPoint_fromView_(event.locationInWindow(), nil);
                    if x.is_sign_negative()
                        || y.is_sign_negative()
                        || x > view_rect.size.width
                        || y > view_rect.size.height
                    {
                        return;
                    }

                    view.convertRectToBacking(NSRect::new(
                        NSPoint {
                            x,
                            /*

                            IMPORTANT: Flips y coordinate to match application coordinate system...

                            BEFORE: OS coordinate system
                            =============================

                                (0,height)
                                ^
                                |
                                (0,0) -> (width, 0)

                            AFTER: Application coordinate system
                            ====================================

                                (0,0) -> (width, 0)
                                |
                                v
                                (0,height)

                            */
                            y: view_rect.size.height - y,
                        },
                        NSSize::new(0.0, 0.0),
                    ))
                    .origin
                };
                let _manager = unsafe {
                    &mut *(*this.get_ivar::<*mut c_void>("applicationManager")
                        as *mut ApplicationManager)
                };
            }
            decl.add_method(
                sel!(acceptsFirstResponder),
                accepts_first_responder as extern "C" fn(&Object, Sel) -> BOOL,
            );
            decl.add_method(
                sel!(mouseMoved:),
                on_mouse_moved as extern "C" fn(&Object, Sel, id),
            );
            decl.add_ivar::<*mut c_void>(&"applicationManager");
            let viewclass = decl.register();
            let view: id = msg_send![viewclass, alloc];
            let () = msg_send![view, init];
            let self_ptr: *mut ApplicationManager = &mut **self;
            (&mut *view).set_ivar::<*mut c_void>("applicationManager", self_ptr as *mut c_void);
            view.setWantsLayer(YES);
            view.setLayer(std::mem::transmute(self.renderer.layer.as_ref()));
            nswindow.setContentView_(view);
            nswindow.setInitialFirstResponder_(view);
        }
    }

    fn init_window_event_handlers(self: &mut Box<Self>, nswindow: *mut Object) {
        let manager_ptr: *mut ApplicationManager = &mut **self;

        #[allow(non_snake_case)]
        extern "C" fn on_nswindow_resize(this: &Object, _: Sel, notification: *mut Object) {
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
            let manager = unsafe {
                &mut *(*this.get_ivar::<*mut c_void>("applicationManager")
                    as *mut ApplicationManager)
            };
            manager
                .renderer
                .render(Size::from_array([width as Unit, height as Unit]));
        }

        unsafe {
            #[allow(non_camel_case_types)]
            type id = cocoa::base::id; // Used by code generated by `delegate!` macro.
            debug_assert_objc_class(nswindow, &"NSWindow").setDelegate_(delegate!("WindowDelegate", {
                // Instance Variables (Retrieved using `this.get_ivar()`)
                applicationManager: *mut c_void = manager_ptr as *mut c_void,
                // Callback Functions
                (windowDidResize:) => on_nswindow_resize as extern fn(&Object, Sel, *mut Object),
                (windowDidBecomeMain:) => on_nswindow_resize as extern fn(&Object, Sel, *mut Object)
            }));
        }
    }
}

pub fn launch_application() {
    let (app, _component_manager) = autoreleasepool(|| unsafe {
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
                                .init_str(&format!("Quit {APP_NAME}"))
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

        let window = NSWindow::alloc(nil)
            .initWithContentRect_styleMask_backing_defer_(
                // NSRect::new(NSPoint::new(0., 0.), NSSize::new(2560.0, 1024.0)),
                NSRect::new(NSPoint::new(0., 0.), NSSize::new(640.0, 640.0)),
                // TODO: Consider rendering a custom title or no title bar at all
                //       To maintain resizability...
                //       - use NSWindowStyleMask::NSResizableWindowMask | NSWindowStyleMask::NSFullSizeContentViewWindowMask,
                //       - window.canBecomeKeyWindow();
                //       - window.canBecomeMainWindow();
                NSWindowStyleMask::NSTitledWindowMask | NSWindowStyleMask::NSResizableWindowMask,
                NSBackingStoreBuffered,
                YES,
            )
            .autorelease();
        window.setAcceptsMouseMovedEvents_(YES);
        window.setPreservesContentDuringLiveResize_(false);
        window.setTitle_(NSString::alloc(nil).init_str(APP_NAME).autorelease());
        window.makeKeyAndOrderFront_(nil);

        (app, ApplicationManager::from_nswindow(window))
    });
    unsafe {
        app.activateIgnoringOtherApps_(true);
        app.run();
    }
}
