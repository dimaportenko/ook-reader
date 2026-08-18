use std::{cell::RefCell, path::PathBuf};

use objc2::{
    define_class, msg_send,
    rc::Retained,
    runtime::{NSObject, NSObjectProtocol, ProtocolObject},
    DefinedClass, MainThreadMarker, MainThreadOnly,
};
use objc2_foundation::{NSArray, NSString, NSURL};
use objc2_ui_kit::{UIDocumentPickerDelegate, UIDocumentPickerViewController, UIViewController};
use objc2_uniform_type_identifiers::UTType;

use crate::renderer::tao::{platform::ios::WindowExtIOS, window::Window};

define_class!(
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "OokDocumentPickerDelegate"]
    #[ivars = Box<dyn Fn(Vec<PathBuf>)>]
    struct PickerDelegate;

    unsafe impl NSObjectProtocol for PickerDelegate {}

    unsafe impl UIDocumentPickerDelegate for PickerDelegate {
        #[unsafe(method(documentPicker:didPickDocumentsAtURLs:))]
        fn documentPicker_didPickDocumentsAtURLs(
            &self,
            _picker: &UIDocumentPickerViewController,
            urls: &NSArray<NSURL>,
        ) {
            let paths = urls
                .iter()
                .filter_map(|url| url.path())
                .map(|path| PathBuf::from(path.to_string()))
                .collect();

            (self.ivars())(paths);
        }
    }
);

impl PickerDelegate {
    fn new(mtm: MainThreadMarker, handle: impl Fn(Vec<PathBuf>) + 'static) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(Box::new(handle) as Box<dyn Fn(Vec<PathBuf>)>);
        unsafe { msg_send![super(this), init] }
    }
}

thread_local! {
    static DELEGATE: RefCell<Option<Retained<PickerDelegate>>> = const { RefCell::new(None) };
}

pub(crate) fn pick_epubs(window: &Window, handle: impl Fn(Vec<PathBuf>) + 'static) {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };
    let Some(epub) = UTType::typeWithFilenameExtension(&NSString::from_str("epub")) else {
        return;
    };
    let root = window.ui_view_controller().cast::<UIViewController>();
    let Some(root) = (unsafe { root.as_ref() }) else {
        return;
    };

    let picker = UIDocumentPickerViewController::initForOpeningContentTypes_asCopy(
        UIDocumentPickerViewController::alloc(mtm),
        &NSArray::from_retained_slice(&[epub]),
        true,
    );
    picker.setAllowsMultipleSelection(true);

    let delegate = PickerDelegate::new(mtm, handle);
    picker.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
    DELEGATE.with(|slot| slot.replace(Some(delegate)));

    root.presentViewController_animated_completion(&picker, true, None);
}
