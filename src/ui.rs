use std::sync::{Arc, Mutex};

use adw::glib::Propagation;
use gtk::{CssProvider, Grid};
use gtk::glib::ExitCode;
use gtk::prelude::{ApplicationExt, ApplicationExtManual, BoxExt, ButtonExt, EditableExt, EntryExt, GridExt, GtkWindowExt, PopoverExt, WidgetExt};
use strum::IntoEnumIterator;

use cpu_test::CpuTestMethod;

use crate::{AppState, cpu_test};

pub fn start_ui_application(app_state: Arc<Mutex<AppState>>) -> ExitCode {
    let application = adw::Application::builder()
        .application_id("rouhim.pbo-assistant")
        .build();
    application.connect_activate(move |application| {
        let app_state = app_state.clone();
        build_ui(application, app_state);
    });
    application.connect_startup(|_| load_css());
    application.run()
}

fn load_css() {
    // Load the CSS file and add it to the provider
    let provider = CssProvider::new();
    provider.load_from_string(include_str!("../styles/style.css"));

    // Add the provider to the default screen
    gtk::style_context_add_provider_for_display(
        &adw::gdk::Display::default().expect("Could not connect to a display."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
}

fn build_ui(application: &adw::Application, app_state: Arc<Mutex<AppState>>) {
    let window = gtk::ApplicationWindow::new(application);

    // Create grid layout and add it to the app state
    let cpu_core_grid = gtk::Grid::builder()
        .row_spacing(5)
        .column_spacing(5)
        .margin_end(10)
        .margin_start(10)
        .margin_top(10)
        .margin_bottom(10)
        .hexpand(true)
        .vexpand(true)
        .build();

    window.set_title(Some("PBO-Assistant"));
    window.set_default_size(1024, 768);
    window.set_titlebar(Some(&build_header_bar(&app_state, &cpu_core_grid)));

    // Create Vertical base layout
    let base_layout = gtk::Box::new(gtk::Orientation::Vertical, 0);

    base_layout.append(&cpu_core_grid);

    window.set_child(Some(&base_layout));

    window.present();
}

fn build_header_bar(app_state: &Arc<Mutex<AppState>>, cpu_core_grid: &Grid) -> gtk::HeaderBar {
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
    let app_state_for_start = app_state.clone();
    let cpu_core_grid = cpu_core_grid.clone();
    start_test_button.connect_clicked(move |_| {
        ;
        start_test(&app_state_for_start, &cpu_core_grid);
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
    popover.set_child(Some(&build_config_layout(&app_state.clone())));
    popover.set_parent(&settings_button);
    settings_button.connect_clicked(move |_| {
        popover.popup();
    });

    header_bar
}

fn build_config_layout(app_state: &Arc<Mutex<AppState>>) -> gtk::Box {
    let app_state_locked = app_state.lock().unwrap();
    let time_per_core = app_state_locked.test_config.duration_per_core.clone();
    let cores_to_test = app_state_locked.test_config.cores_to_test.clone();
    let active_test_methods = app_state_locked.test_config.test_methods.clone();

    let config_layout = gtk::Box::new(gtk::Orientation::Vertical, 10);

    // Add textfield for time
    let time_textfield = gtk::Entry::new();
    time_textfield.set_text(&time_per_core);
    time_textfield.set_placeholder_text(Some("10m"));
    time_textfield.set_tooltip_text(Some("Duration per core. E.g. 10m, 20s, 1h"));
    config_layout.append(&build_settings_row(
        "Runtime per Core",
        time_textfield.clone().into(),
    ));
    let app_state_for_time = app_state.clone();
    time_textfield.connect_changed(move |textfield| {
        let mut app_state = app_state_for_time.lock().unwrap();
        app_state.test_config.duration_per_core = textfield.text().to_string();
    });

    // Add textfield for cores
    let cores_textfield = gtk::Entry::new();
    cores_textfield.set_text(
        &cores_to_test
            .iter()
            .map(|x| x.to_string())
            .collect::<Vec<String>>()
            .join(","),
    );
    cores_textfield.set_tooltip_text(Some("Comma separated list of cores to test. E.g. 0,1,2,3"));
    config_layout.append(&build_settings_row(
        "Cores to Test",
        cores_textfield.clone().into(),
    ));
    let app_state_for_cores = app_state.clone();
    cores_textfield.connect_changed(move |textfield| {
        let mut app_state = app_state_for_cores.lock().unwrap();
        app_state.test_config.cores_to_test = textfield
            .text()
            .split(',')
            .map(|x| x.parse().unwrap())
            .collect();
    });

    // Load all enum values from test method enum
    let all_test_methods = cpu_test::CpuTestMethod::iter().collect::<Vec<CpuTestMethod>>();

    // For each test method add a switch
    for test_method in all_test_methods {
        let switch = gtk::Switch::new();
        switch.set_active(active_test_methods.contains(&test_method));
        switch.set_tooltip_text(Some("If multiple test methods are selected, they run one after the other, divided by the duration per core"));
        let app_state_for_test_method = app_state.clone();
        switch.connect_state_set(move |_, new_state| {
            let mut app_state = app_state_for_test_method.lock().unwrap();
            let test_methods = &mut app_state.test_config.test_methods;
            if new_state {
                test_methods.push(test_method);
            } else {
                test_methods.retain(|x| x != &test_method);
            }
            Propagation::Proceed
        });

        config_layout.append(&build_settings_row(
            &test_method.to_string(),
            switch.clone().into(),
        ));
    }

    config_layout
}

/// Builds a row with a label and a widget
/// The label is left aligned and the widget is right aligned
/// The widget is expected to be a gtk::Widget
/// The label_text is the text that will be displayed on the left side
/// # Parameters
/// * `label_text` - The text that will be displayed on the left side
/// * `widget` - The generic widget that will be displayed on the right side
fn build_settings_row(label_text: &str, widget: gtk::Widget) -> gtk::Box {
    // Horizontal layout with label
    let horizontal_layout = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    horizontal_layout.set_hexpand(true);

    // Create the label
    let label = gtk::Label::new(Some(&label_text));

    // Create the spacer
    let spacer = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);

    horizontal_layout.append(&label);
    horizontal_layout.append(&spacer);
    horizontal_layout.append(&widget);
    horizontal_layout
}

fn start_test(app_state: &Arc<Mutex<AppState>>, cpu_core_grid: &Grid) {
    println!("Starting test");

    fix_actual_cores_to_test(app_state);

    cpu_test::initialize_response(&app_state.clone());

    // Render loop for cpus
    build_render_loop(app_state.clone(), cpu_core_grid);

    //cpu_test::run(app_state.clone());
}

fn fix_actual_cores_to_test(app_state: &Arc<Mutex<AppState>>) {
    let mut app_state_locked = app_state.lock().unwrap();

    // Gets the actual cores to test, if the user entered more cores than available, the function will return the available cores
    // Also alternates the cores
    let cores_to_test = app_state_locked.test_config.cores_to_test.clone();
    let cores_to_test = cpu_test::get_cores_to_test(cores_to_test, cpu_test::get_physical_cores());
    app_state_locked.test_config.cores_to_test.clone_from(&cores_to_test);
}

fn build_render_loop(app_state: Arc<Mutex<AppState>>, cpu_core_grid: &Grid) {
    // First add for each cpu core a layout to the cpu grid view
    let cores_to_test = app_state.lock().unwrap().test_config.cores_to_test.clone();

    // iterate over cores to test and create a layout for each core
    for core in cores_to_test.iter() {
        let core_layout = build_core_layout(&core);
        cpu_core_grid.attach(&core_layout, *core as i32, 0, 1, 1);
    }

    // Then start a dedicated thread that checks every second the app state
    // And adjust the cpu layout accordingly
}

fn build_core_layout(core: &&usize) -> gtk::Box {
    let core_layout = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .margin_top(5)
        .margin_bottom(5)
        .margin_start(5)
        .margin_end(5)
        .height_request(100)
        .width_request(100)
        .css_classes(vec!["core-layout"])
        .build();


    let core_label = gtk::Label::new(Some(&format!("Core {}", core)));
    core_layout.append(&core_label);

    core_layout
}
