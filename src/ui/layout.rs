use crate::utils;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame, DefaultTerminal,
};
use crossterm::event::{self, Event, KeyCode};
use std::io;

// 应用状态（规范）
#[derive(Default)]
pub struct App {
    exit: bool,
}

impl App {
    /// 运行主循环直到用户退出
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> anyhow::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    /// 绘制 UI（纯渲染，不修改状态）
    fn draw(&self, frame: &mut Frame) {
        let area = frame.area();
        self.draw_welcome_ui(frame, area);
    }

    /// 处理键盘事件
    fn handle_events(&mut self) -> io::Result<()> {
        if event::poll(std::time::Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => self.exit = true,
                    _ => {}
                }
            }
        }
        Ok(())
    }
}

// UI 渲染逻辑
impl App {
    fn draw_welcome_ui(&self, frame: &mut Frame, area: Rect) {
        // 主布局：内容区（固定17行）+ 底部提示
        let main_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(17),  // 固定高度17行
                Constraint::Min(1),      // 至少1行，实际会占满剩余空间用于输入
            ])
            .split(area);

        // 左右分栏（固定17行内再分割）
        let panels = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(35),
                Constraint::Percentage(65),
            ])
            .split(main_chunks[0]); // 使用固定的17行区域

        self.draw_left_panel(frame, panels[0]);
        self.draw_right_panel(frame, panels[1]);
        self.draw_bottom_prompt(frame, main_chunks[1]); // 底部输入区域
    }

    /// 左侧面板
    fn draw_left_panel(&self, frame: &mut Frame, area: Rect) {
        let border = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(106, 176, 76)));

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Length(10),
                Constraint::Min(1),
            ])
            .split(area);

        // 标题
        let title = Paragraph::new(Line::from(
            Span::styled(format!("Omega Code v{}",
                                 utils::metadata::get_omega_version()[0].version),
                         Style::default().fg(Color::Rgb(106, 176, 76)))
        )).block(border.clone());
        frame.render_widget(title, chunks[0]);

        // 欢迎 + 图标
        let welcome = Paragraph::new(vec![
            Line::from("Welcome back!"),
            Line::from(r#" ______     __    __     ______     ______     ______    "#),
            Line::from(r#"/\  __ \   /\ "-./  \   /\  ___\   /\  ___\   /\  __ \   "#),
            Line::from(r#"\ \ \/\ \  \ \ \-./\ \  \ \  __\   \ \ \__ \  \ \  __ \  "#),
            Line::from(r#" \ \_____\  \ \_\ \ \_\  \ \_____\  \ \_____\  \ \_\ \_\ "#),
            Line::from(r#"  \/_____/   \/_/  \/_/   \/_____/   \/_____/   \/_/\/_/ "#),
        ]).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(welcome, chunks[1]);

        // 底部信息
        let info = Paragraph::new(vec![
            Line::from(Span::styled("Sonnet 4.5 · Max 20x", Style::default().fg(Color::DarkGray))),
            Line::from(Span::styled("/users/meaghan/code/apps", Style::default().fg(Color::DarkGray))),
        ]).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(info, chunks[2]);

        frame.render_widget(border, area);
    }

    /// 右侧面板：最近活动 + 新功能
    fn draw_right_panel(&self, frame: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(area);

        // 最近活动
        let activity_block = Block::default()
            .title(Span::styled("Recent activity", Style::default().fg(Color::Rgb(106, 176, 76))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(106, 176, 76)));

        let activity = Paragraph::new(vec![
            Line::from(vec![Span::styled("1m ago   ", Color::DarkGray), Span::raw("Updated project memory")]),
            Line::from(vec![Span::styled("8m ago   ", Color::DarkGray), Span::raw("Updated claw'd feet")]),
            Line::from(vec![Span::styled("2d ago   ", Color::DarkGray), Span::raw("Add new words to spinner")]),
            Line::from(vec![Span::styled("1w ago   ", Color::DarkGray), Span::raw("Update unit tests")]),
            Line::from(vec![Span::styled("2w ago   ", Color::DarkGray), Span::raw("Created new project")]),
            Line::from(vec![Span::styled("1y ago   ", Color::DarkGray), Span::raw("Add new words to spinner")]),
            Line::from(Span::styled("... /resume for more", Color::DarkGray)),
        ]).block(activity_block);
        frame.render_widget(activity, chunks[0]);

        // 新功能
        let new_block = Block::default()
            .title(Span::styled("How to use", Style::default().fg(Color::Rgb(106, 176, 76))))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Rgb(106, 176, 76)));

        let new_content = Paragraph::new(vec![
            Line::from("use '/{command}' to separate commands"),
            Line::from("use '@{fileName}' to reference the file"),
            Line::from("can drag a file to the window to reference an attachment to ask a question"),
            Line::from("use '@{fileName}#{lineNumber [- lineNumber]}' to reference lines in files"),
            Line::from("open http://localhost:5200 to manage your Omega Code"),
            Line::from(Span::styled("... /help for more", Color::DarkGray)),
        ]).block(new_block);
        frame.render_widget(new_content, chunks[1]);
    }

    /// 底部提示 - 输入区域
    fn draw_bottom_prompt(&self, frame: &mut Frame, area: Rect) {
        let prompt = Paragraph::new(Span::styled(
            "Try \"edit <filepath> to ...\"",  // 这里可以改为实际的输入框或提示
            Style::default().fg(Color::DarkGray)
        ));
        frame.render_widget(prompt, area);
    }
}

// 标准主函数（极简、规范）
fn main() -> anyhow::Result<()> {
    let mut terminal = ratatui::init();
    let mut app = App::default();
    let result = app.run(&mut terminal);
    ratatui::restore();
    result
}