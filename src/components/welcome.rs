use ratatui_kit::{
    Component,
    ComponentDrawer,
    ComponentUpdater,
    Hooks,
    Props,
    UseEvents,
    UseState,
};

use ratatui_kit::crossterm::event::{Event, KeyCode};

use ratatui_kit::ratatui::{
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};

#[derive(Props)]
pub struct WelcomeProps {
    pub version: Option<String>,
    pub style: Style,
    pub menu_style: Style,
    pub show_menu: bool,
}

impl Default for WelcomeProps {
    fn default() -> Self {
        Self {
            version: None,
            style: Style::default(),
            menu_style: Style::default(),
            show_menu: true,
        }
    }
}

pub struct Welcome {
    selected_index: usize,

    // props cache
    style: Style,
    menu_style: Style,
    version: Option<String>,
    show_menu: bool,
}

impl Component for Welcome {
    type Props<'a> = WelcomeProps;

    fn new(props: &Self::Props<'_>) -> Self {
        Self {
            selected_index: 0,

            style: props.style,
            menu_style: props.menu_style,
            version: props.version.clone(),
            show_menu: props.show_menu,
        }
    }

    fn update(
        &mut self,
        props: &mut Self::Props<'_>,
        mut hooks: Hooks,
        _updater: &mut ComponentUpdater,
    ) {
        // sync props
        self.style = props.style;
        self.menu_style = props.menu_style;
        self.version = props.version.clone();
        self.show_menu = props.show_menu;

        let mut state_index = hooks.use_state(|| self.selected_index);

        hooks.use_events(move |event| {
            if let Event::Key(key) = event {
                match key.code {
                    KeyCode::Up => {
                        state_index.set(
                            state_index
                                .get()
                                .saturating_sub(1),
                        );
                    }

                    KeyCode::Down => {
                        let current = state_index.get();

                        state_index.set((current + 1).min(2));
                    }

                    KeyCode::Enter => {
                        match state_index.get() {
                            0 => {
                                todo!("打开配置");
                            }

                            1 => {
                                todo!("开启新项目");
                            }

                            2 => {
                                todo!("退出程序");
                            }

                            _ => {}
                        }
                    }

                    _ => {}
                }
            }
        });

        self.selected_index = state_index.get();
    }

    fn draw(&mut self, drawer: &mut ComponentDrawer<'_, '_>) {
        let area = drawer.area;
        let buf = drawer.buffer_mut();

        // ASCII LOGO
        let ascii_lines = [
            " ██████╗ ███╗   ███╗███████╗ ██████╗  █████╗",
            "██╔═══██╗████╗ ████║██╔════╝██╔════╝ ██╔══██╗",
            "██║   ██║██╔████╔██║█████╗  ██║  ███╗███████║",
            "██║   ██║██║╚██╔╝██║██╔══╝  ██║   ██║██╔══██║",
            "╚██████╔╝██║ ╚═╝ ██║███████╗╚██████╔╝██║  ██║",
            " ╚═════╝ ╚═╝     ╚═╝╚══════╝ ╚═════╝ ╚═╝  ╚═╝",
            "",
            " ██████╗ ██████╗ ██████╗ ███████╗",
            "██╔════╝██╔═══██╗██╔══██╗██╔════╝",
            "██║     ██║   ██║██║  ██║█████╗  ",
            "██║     ██║   ██║██║  ██║██╔══╝  ",
            "╚██████╗╚██████╔╝██████╔╝███████╗",
            " ╚═════╝ ╚═════╝ ╚═════╝ ╚══════╝",
        ];

        let version = self
            .version
            .clone()
            .unwrap_or_else(|| {
                env!("CARGO_PKG_VERSION").to_string()
            });

        let welcome_text = t!("welcome.text");

        let menu_items = [
            t!("welcome.menu_items.0"),
            t!("welcome.menu_items.1"),
            t!("welcome.menu_items.2"),
        ];

        let mut lines = Vec::new();

        // logo
        for line in ascii_lines {
            lines.push(
                Line::from(
                    Span::styled(
                        line,
                        self.style,
                    )
                )
            );
        }

        lines.push(Line::from(""));

        // version
        lines.push(
            Line::from(
                Span::styled(
                    t!("welcome.version", version = version),
                    self.style.add_modifier(Modifier::DIM),
                )
            )
        );

        lines.push(Line::from(""));

        // welcome text
        lines.push(
            Line::from(
                Span::styled(
                    welcome_text,
                    self.style,
                )
            )
        );

        lines.push(Line::from(""));

        // menu title
        lines.push(
            Line::from(
                Span::styled(
                    t!("welcome.menu"),
                    self.style.add_modifier(Modifier::BOLD),
                )
            )
        );

        lines.push(Line::from(""));

        // menu
        if self.show_menu {
            let max_index = menu_items
                .len()
                .saturating_sub(1);

            self.selected_index =
                self.selected_index.min(max_index);

            for (i, item) in menu_items.iter().enumerate() {
                let selected =
                    i == self.selected_index;

                let prefix = if selected {
                    "→ "
                } else {
                    "  "
                };

                let style = if selected {
                    self.menu_style.add_modifier(
                        Modifier::BOLD
                            | Modifier::REVERSED,
                    )
                } else {
                    self.style
                };

                lines.push(
                    Line::from(
                        Span::styled(
                            format!("{prefix}• {item}"),
                            style,
                        )
                    )
                );
            }
        }

        Paragraph::new(lines).render(area, buf);
    }
}