use std::sync::OnceLock;

use glib::subclass::Signal;
use glib_macros::Properties;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use super::*;

impl CoreBoxLayout {
    pub fn new() -> Self {
        Self::default()
    }
}

#[derive(Default, Properties)]
#[properties(wrapper_type = super::CoreBoxLayout)]
pub struct CoreBoxLayout;

#[glib::object_subclass]
impl ObjectSubclass for CoreBoxLayout {
    const NAME: &'static str = "CoreBoxLayout";
    type Type = super::CoreBoxLayout;
    type ParentType = gtk::Box;
}

// Trait shared by all GObjects
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


impl WidgetImpl for CoreBoxLayout {}

impl BoxImpl for CoreBoxLayout {}