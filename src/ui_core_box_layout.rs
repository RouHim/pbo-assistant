#![allow(clippy::all)]

use gio::prelude::*;
use glib::subclass::prelude::*;
use glib::Properties;

// UI model!
// Despite the name, it can represent a playlist as well
// TODO: https://github.com/gtk-rs/gtk4-rs/blob/master/book/listings/g_object_signals/2/custom_button/imp.rs
glib::wrapper! {
    pub struct CoreBoxLayout(ObjectSubclass<imp::CoreBoxLayout>);
}

impl CoreBoxLayout {
    pub fn new(
        core_id: u32,
    ) -> CoreBoxLayout {
        glib::Object::builder()
            .property("core_id", core_id)
            .build()
    }
}

mod imp {

    use super::*;

    use std::cell::{Cell, RefCell};
    use std::sync::OnceLock;
    use glib::subclass::Signal;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::CoreBoxLayout)]
    pub struct CoreBoxLayout {
        #[property(get, set)]
        core_id: RefCell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CoreBoxLayout {
        const NAME: &'static str = "CoreBoxLayout";
        type Type = super::CoreBoxLayout;
        type ParentType = gtk::Grid;
    }

    #[glib::derived_properties]
    impl ObjectImpl for CoreBoxLayout {
        fn signals() -> &'static [Signal] {
            static SIGNALS: OnceLock<Vec<Signal>> = OnceLock::new();
            SIGNALS.get_or_init(|| {
                vec![Signal::builder("update")
                    .param_types([i32::static_type()])
                    .build()]
            })
        }
    }
}