use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use gtk::glib::ExitCode;
use gtk::prelude::{ApplicationExt, ApplicationExtManual, BoxExt, ButtonExt, EditableExt, GtkWindowExt, PopoverExt, WidgetExt};

use crate::{AppState, cpu_test};
use crate::cpu_test::CpuTestResponse;

pub fn start_ui_application(app_state: Arc<Mutex<AppState>>) -> ExitCode {
    let application = adw::Application::builder()
        .application_id("rouhim.pbo-assistant")
        .build();
    application.connect_activate(move |application| {
        let app_state = app_state.clone();
        build_ui(application, app_state);
    });
    application.run()
}

fn build_ui(application: &adw::Application, app_state: Arc<Mutex<AppState>>) {
    let window = gtk::ApplicationWindow::new(application);

    window.set_title(Some("PBO-Assistant"));
    window.set_default_size(1024, 768);
    window.set_titlebar(Some(&build_header_bar()));

    // Create Vertical base layout
    let base_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

    // Create grid layout
    let core_grid = gtk::Grid::new();

    base_layout.append(&core_grid);

    window.set_child(Some(&base_layout));

    window.present();
}

fn build_header_bar() -> gtk::HeaderBar {
    let header_bar = gtk::HeaderBar::new();
    header_bar.set_title_widget(Some(&gtk::Label::new(Some("PBO-Assistant"))));
    header_bar.set_show_title_buttons(true);


    // Add start test button to gtk header bar
    let start_test_button = gtk::Button::builder()
        .icon_name("media-playback-start-symbolic")
        .tooltip_text("Start Test")
        .margin_start(0)
        .css_classes(vec!["suggested-action"])
        .build();
    start_test_button.connect_clicked(move |_| {
        start_test();
    });
    header_bar.pack_start(&start_test_button);

    // Add settings sandwich button to gtk header bar
    let settings_button = gtk::Button::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Settings")
        .margin_start(10)
        .build();
    header_bar.pack_end(&settings_button);

    // Build settings popover
    let popover = gtk::Popover::new();
    popover.set_child(Some(&build_config_layout()));
    popover.set_parent(&settings_button);
    settings_button.connect_clicked(move |_| {
        popover.popup();
    });

    header_bar
}

fn build_config_layout() -> gtk::Box {
    let config_layout = gtk::Box::new(gtk::Orientation::Vertical, 10);

    // Add textfield for time
    let time_textfield = gtk::Entry::new();
    time_textfield.set_text("20s");
    config_layout.append(&time_textfield);

    // Add textfield for cores
    let cores_textfield = gtk::Entry::new();
    cores_textfield.set_text("0,1,2,3");
    config_layout.append(&cores_textfield);

    // Add multiselect or combobox for test methods
    let test_methods = gtk::ComboBoxText::new();
    test_methods.append_text("Prime95");
    test_methods.append_text("YCruncher");
    config_layout.append(&test_methods);

    config_layout
}

fn start_test() {
    println!("Starting test");

    let core_to_test = "";

    let cores_to_test = cpu_test::get_cores_to_test(core_to_test, cpu_test::get_physical_cores());

    let config = cpu_test::CpuTestConfig {
        duration_per_core: "20s".to_string(),
        cores_to_test,
        test_methods: vec![
            cpu_test::CpuTestMethod::Prime95,
            cpu_test::CpuTestMethod::YCruncher,
        ],
    };

    // Holds the global state of the test results
    let test_results: Arc<Mutex<HashMap<usize, CpuTestResponse>>> =
        Arc::new(Mutex::new(HashMap::new()));

    cpu_test::initialize_response(&config, &test_results, &config.duration_per_core);

    cpu_test::run(config, test_results.clone());

    let rest_results = test_results.lock().unwrap();
    let mut values: Vec<&CpuTestResponse> = rest_results.values().collect();
    values.sort_by(|a, b| a.core_id.cmp(&b.core_id));
    println!();
    println!();
    println!();
    for cpu_result in values {
        println!("{:?}", cpu_result);
    }
}
