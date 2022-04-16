use objc::runtime::Object;

#[inline(always)]
pub fn debug_assert_objc_class(
    #[allow(unused_variables)] obj: *mut Object,
    #[allow(unused_variables)] class_name: &'static str,
) -> *mut Object {
    #[cfg(debug_assertions)]
    {
        use objc::runtime::{Class, BOOL, YES};
        let class = Class::get(class_name);
        let result: BOOL = unsafe { msg_send![obj, isKindOfClass: class] };
        assert_eq!(
            result, YES,
            "Expected Objective-C object to be kind of class {class_name}"
        );
    }
    obj
}
