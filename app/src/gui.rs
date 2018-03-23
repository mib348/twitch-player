extern crate qt_core;
extern crate qt_gui;
extern crate qt_widgets;
extern crate cpp_utils;

use self::qt_core::connection::Signal;
use self::qt_core::core_application::CoreApplication;
use self::qt_core::flags::{Flags, FlaggableEnum};
use self::qt_core::list::ListCInt;
use self::qt_core::qt::GlobalColor;
use self::qt_core::qt::WindowState;
use self::qt_core::slots::SlotNoArgs;
use self::qt_core::string::String as QString;
use self::qt_core::timer::Timer;
use self::qt_gui::color::Color;
use self::qt_gui::key_sequence::{KeySequence, StandardKey};
use self::qt_gui::palette::{Palette, ColorRole};
use self::qt_gui::window::Window;
use self::qt_widgets::application::Application;
use self::qt_widgets::main_window::MainWindow;
use self::qt_widgets::message_box::MessageBox;
use self::qt_widgets::shortcut::Shortcut;
use self::qt_widgets::splitter::Splitter;
use self::qt_widgets::widget::Widget;
use self::qt_widgets::input_dialog::InputDialog;
use self::cpp_utils::StaticCast;

use types::{GuiReceiver, PlayerSender, PlayerMessage};

use std::ptr;

type WidgetRatios = (i32, i32);

const INITIAL_WIDGETS_RATIOS: WidgetRatios = (80, 20);
const FULL_SCREEN_RATIOS: WidgetRatios = (100, 0);

fn critical(title: &str, text: &str) {
    let args = (
        ptr::null_mut(),
        &QString::from_std_str(title.to_string()),
        &QString::from_std_str(text.to_string())
    );
    unsafe { MessageBox::critical(args) };
}

fn get_text(title: &str, text: &str) -> String {
    let args = (
        ptr::null_mut(),
        &QString::from_std_str(title.to_string()),
        &QString::from_std_str(text.to_string())
    );
    unsafe { InputDialog::get_text(args).to_std_string() }
}

unsafe fn widget_from_handle(handle: u64, parent: *mut Widget) -> *mut Widget {
    let window = Window::from_win_id(handle);
    Widget::create_window_container((window, parent))
}

fn toggle_full_screen(widget: &mut Widget) {
    use self::WindowState::FullScreen;

    let state = widget.window_state();
    let new_state = match widget.is_full_screen() {
        true =>  state & Flags::from_int(!(FullScreen.to_flag_value())),
        false => state | Flags::from_enum(FullScreen)
    };
    widget.set_window_state(new_state);
}

fn resize_splitter(splitter: &mut Splitter, ratios: WidgetRatios) {
    let mut sizes = ListCInt::new(());
    sizes.append(&ratios.0);
    sizes.append(&ratios.1);
    splitter.set_sizes(&sizes)
}

fn get_widgets_ratios(splitter: &Splitter) -> WidgetRatios {
    let sizes = splitter.sizes();
    (*sizes.at(0), *sizes.at(1))
}

pub fn run(messages_in: GuiReceiver, messages_out: PlayerSender) -> ! {
    let on_exit = SlotNoArgs::new(|| {
        use types::PlayerMessage::AppQuit;

        messages_out.send(AppQuit).unwrap_or_default();
        if let Err(error) = messages_in.recv() {
            eprintln!("Error: {:?}", error)
        }
    });

    Application::create_and_exit(|app| unsafe {
        let core_app: &CoreApplication = app.static_cast();
        core_app.signals()
            .about_to_quit()
            .connect(&on_exit);

        // Setup window layout
        let mut main_window = MainWindow::new();
        let main_window_ptr = main_window.static_cast_mut() as *mut Widget;

        let mut splitter = Splitter::new(());
        let splitter_ptr = splitter.as_mut_ptr();

        let mut palette = Palette::new(());
        palette.set_color((ColorRole::Window, &Color::new(GlobalColor::Black)));
        main_window.set_palette(&palette);

        main_window.set_central_widget(splitter.static_cast_mut());
        main_window.resize((1280, 720));
        main_window.show_maximized();

        // Player messages polling
        let mut poll_timer = Timer::new();
        let mut poll_messages = SlotNoArgs::default();
        poll_timer.signals()
            .timeout()
            .connect(&poll_messages);
        poll_timer.start(250);

        // Fullscreen handling
        let fullscreen_seq = KeySequence::new(StandardKey::FullScreen);
        let fullscreen_shortcut = Shortcut::new((&fullscreen_seq, main_window_ptr));
        let mut on_fullscreen = SlotNoArgs::default();
        fullscreen_shortcut.signals()
            .activated()
            .connect(&on_fullscreen);

        // Chat Toggle
        let mut chat_toggled = true;
        let mut widgets_ratios = INITIAL_WIDGETS_RATIOS;
        let toggle_chat_seq = KeySequence::new(StandardKey::MoveToNextWord);
        let toggle_chat_shortcut = Shortcut::new((&toggle_chat_seq, main_window_ptr));
        let mut on_toggle_chat = SlotNoArgs::default();
        toggle_chat_shortcut.signals()
            .activated()
            .connect(&on_toggle_chat);

        // Change channel
        let change_channel_seq = KeySequence::new(StandardKey::Open);
        let change_channel_shortcut = Shortcut::new((&change_channel_seq, main_window_ptr));
        let mut on_change_channel = SlotNoArgs::default();
        change_channel_shortcut.signals()
            .activated()
            .connect(&on_change_channel);

        // Slots
        poll_messages.set(|| {
            use types::GuiMessage::*;
            let splitter = &mut *splitter_ptr;

            if let Ok(message) = messages_in.try_recv() {
                match message {
                    PlayerError(reason) => {
                        critical("Fatal player error", &reason);
                        CoreApplication::quit();
                    },
                    Handles(player, chat) => {
                        splitter.add_widget(widget_from_handle(player, main_window_ptr));
                        splitter.add_widget(widget_from_handle(chat, main_window_ptr));
                        resize_splitter(splitter, INITIAL_WIDGETS_RATIOS);
                    },
                    _ => { }
                }
            }
        });

        on_fullscreen.set(|| toggle_full_screen(&mut *main_window_ptr));

        on_toggle_chat.set(|| {
            chat_toggled = !chat_toggled;
            let ratios = match chat_toggled {
                true  => widgets_ratios,
                false => {
                    widgets_ratios = get_widgets_ratios(&*splitter_ptr);
                    FULL_SCREEN_RATIOS
                }
            };
            resize_splitter(&mut *splitter_ptr, ratios);
        });

        on_change_channel.set(|| {
            let channel = get_text("Change channel", "New channel");
            if channel.len() > 0 {
                let order = PlayerMessage::ChangeChannel(channel);
                messages_out.send(order).unwrap_or_default();
            }
        });

        Application::exec()
    })
}
