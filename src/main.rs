use std::{error::Error, io::Stdout, rc::Rc, time::Duration};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{prelude::*, text::Line, widgets::*};
use setup::{restore_terminal, setup_terminal};
use task::*;
use tui_input::backend::crossterm::EventHandler;

#[macro_use]
extern crate ratatui;

#[macro_use]
extern crate anyhow;

mod setup;
mod task;

#[derive(Copy, Clone)]
pub enum TaskStep {
    Title,
    Details,
    Status,
    StatusColor,
}

impl TaskStep {
    pub fn to_message(&self) -> &'static str {
        match self {
            TaskStep::Title => "Please input title",
            TaskStep::Details => "Please input details",
            TaskStep::Status => "Please input status",
            TaskStep::StatusColor => "Please input ansii color code",
        }
    }
}

pub enum InputStatus {
    Empty,
    Controls,
    Request(InputRequestType),
    New,
    Edit,
}

#[derive(Copy, Clone)]
pub enum InputRequestType {
    NewFolder,
    RenameFolder,
    NewTask { step: TaskStep },
    EditTask { step: TaskStep },
    ConfirmDelete,
}

impl InputRequestType {
    pub fn to_message(&self) -> String {
        match self {
            InputRequestType::NewFolder => "Enter the name for the folder".to_string(),
            InputRequestType::RenameFolder => "Enter the new name for the folder".to_string(),
            InputRequestType::NewTask { step } => format!("New Task: {}", step.to_message()),
            InputRequestType::EditTask { step } => format!("Edit Task: {}", step.to_message()),
            InputRequestType::ConfirmDelete => "Are you sure? Y/N".to_string(),
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // Set up the terminal
    let mut terminal = setup_terminal()?;

    // Run main loop
    match run(&mut terminal) {
        Ok(_) => {
            // Take down the terminal
            restore_terminal(&mut terminal)?;
        }
        Err(e) => {
            // Take down the terminal
            restore_terminal(&mut terminal)?;

            // Print the error.
            eprintln!("{}", e);
        }
    }

    Ok(())
}

// The main render function of the engine
fn run(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> anyhow::Result<()> {
    let mut selected_tab = 0;

    let mut selected: Vec<String> = vec![];
    let mut folder = Folder::read_or_create()?;

    let mut input_status = InputStatus::Empty;
    let mut input = tui_input::Input::new("".to_string());

    let mut temp_task = Task::default();

    // Main window loop
    loop {
        let cur_folder = folder.get_folder(selected.clone()).unwrap();

        // Render the frame
        terminal.draw(|frame| {
            let chunks = make_chunks(frame);

            let list = cur_folder.as_list_widget().block(
                Block::default()
                    .title("Tasks")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Plain),
            );
            frame.render_widget(list, chunks.left_menu());

            if let Some(task) = cur_folder.get_selected_task() {
                let border = Block::default()
                    .title("Task Details")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double);

                frame.render_widget(border, chunks.right_menu());

                let status = task.status.to_paragraph().block(
                    Block::default()
                        .title("Status")
                        .borders(border!(TOP))
                        .style(Style::new().fg(Color::White)),
                );

                frame.render_widget(status, chunks.top_detail());

                let details = Paragraph::new(task.task.clone()).block(
                    Block::new()
                        .title("Details")
                        .borders(border!(TOP))
                        .border_type(BorderType::Plain),
                );

                frame.render_widget(details, chunks.detail());

                let misc = Paragraph::new("").block(
                    Block::new()
                        .title("Misc")
                        .borders(border!(TOP))
                        .border_type(BorderType::Plain),
                );

                frame.render_widget(misc, chunks.bottom_detail());
            } else if let Some(folder) = cur_folder.get_selected_folder() {
                let details = folder.as_list_widget().block(
                    Block::default()
                        .title("Inner Tasks")
                        .borders(Borders::ALL)
                        .border_type(BorderType::Double),
                );

                frame.render_widget(details, chunks.right_menu());
            }

            // Render extra
            render_tabs(
                frame,
                &chunks,
                selected_tab,
                vec!["[TAB]  List", "Calendar", "Filter"],
            );

            match input_status {
                InputStatus::Controls => render_help(frame, &chunks),
                InputStatus::New => frame.render_widget(
                    Paragraph::new(vec![Line::from(" <t> TASK "), Line::from(" <f> FOLDER ")])
                        .block(
                            Block::default()
                                .title("Help")
                                .borders(Borders::ALL)
                                .style(Style::new().fg(Color::LightCyan)),
                        ),
                    chunks.message_popup(),
                ),
                InputStatus::Edit => frame.render_widget(
                    Paragraph::new(vec![
                        Line::from(" <d> DETAILS "),
                        Line::from(" <n> NAME "),
                        Line::from(" <s> STATUS "),
                    ])
                    .block(
                        Block::default()
                            .title("Help")
                            .borders(Borders::ALL)
                            .style(Style::new().fg(Color::LightCyan)),
                    ),
                    chunks.message_popup(),
                ),
                _ => {}
            }

            // Finally if input is active, render it.
            if let InputStatus::Request(event) = input_status {
                let popup = Paragraph::new(input.value()).block(
                    Block::default()
                        .title(event.to_message())
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded),
                );

                frame.render_widget(popup, chunks.input_popup());
            }
        })?;

        // Poll Events
        if event::poll(Duration::from_millis(1500))? {
            if let Event::Key(key_event) = event::read()? {
                let key = key_event.code;

                match input_status {
                    InputStatus::Empty => {
                        if key == KeyCode::Char(' ') {
                            input_status = InputStatus::Controls;
                        }
                        match key {
                            KeyCode::Down => cur_folder.adjust_selected(1),
                            KeyCode::Up => cur_folder.adjust_selected(-1),
                            KeyCode::Right => {
                                if let Some(subfolder) = cur_folder.get_selected_folder() {
                                    selected.push(subfolder.name.clone());
                                }
                            }
                            KeyCode::Left => {
                                selected.pop();
                            }
                            KeyCode::Tab => {
                                selected_tab += 1;
                                selected_tab %= 3;
                            }
                            _ => {}
                        }
                    }
                    InputStatus::Controls => match key {
                        KeyCode::Char('q') => break,
                        KeyCode::Char('n') => input_status = InputStatus::New,
                        KeyCode::Char('e') => match cur_folder.get_selected_folder() {
                            Some(_) => {
                                input_status = InputStatus::Request(InputRequestType::RenameFolder)
                            }
                            None => input_status = InputStatus::Edit,
                        },
                        KeyCode::Char('w') => {
                            folder.save()?;
                            input_status = InputStatus::Empty
                        }
                        KeyCode::Char('d') => {
                            input_status = InputStatus::Request(InputRequestType::ConfirmDelete)
                        }
                        _ => input_status = InputStatus::Empty,
                    },
                    InputStatus::New => match key {
                        KeyCode::Char('f') => {
                            input_status = InputStatus::Request(InputRequestType::NewFolder)
                        }
                        KeyCode::Char('t') => {
                            input_status = InputStatus::Request(InputRequestType::NewTask {
                                step: TaskStep::Title,
                            })
                        }
                        _ => input_status = InputStatus::Empty,
                    },
                    InputStatus::Edit => match key {
                        KeyCode::Char('t') => {
                            input_status = InputStatus::Request(InputRequestType::EditTask {
                                step: TaskStep::Title,
                            });
                        }
                        KeyCode::Char('d') => {
                            input_status = InputStatus::Request(InputRequestType::EditTask {
                                step: TaskStep::Details,
                            });
                        }
                        KeyCode::Char('s') => {
                            input_status = InputStatus::Request(InputRequestType::EditTask {
                                step: TaskStep::Status,
                            })
                        }
                        _ => input_status = InputStatus::Empty,
                    },
                    InputStatus::Request(request) => match key {
                        KeyCode::Esc => {
                            input = input.with_value("".to_string());
                            input_status = InputStatus::Empty;
                        }
                        KeyCode::Enter => {
                            match request {
                                InputRequestType::NewFolder => {
                                    cur_folder.new_folder(input.value().to_string());
                                    input_status = InputStatus::Empty
                                }
                                InputRequestType::RenameFolder => {
                                    if let Some(folder) = cur_folder.get_selected_folder() {
                                        folder.name = input.value().to_string()
                                    }
                                    input_status = InputStatus::Empty
                                }
                                InputRequestType::NewTask { step } => match step {
                                    TaskStep::Title => {
                                        temp_task.title = input.value().to_string();
                                        input_status =
                                            InputStatus::Request(InputRequestType::NewTask {
                                                step: TaskStep::Details,
                                            });
                                    }
                                    TaskStep::Details => {
                                        temp_task.task = input.value().to_string();
                                        cur_folder.new_task(temp_task.clone());
                                        input_status = InputStatus::Empty
                                    }
                                    _ => {}
                                },
                                InputRequestType::EditTask { step } => match step {
                                    TaskStep::Title => {
                                        if let Some(cur_task) = cur_folder.get_selected_task() {
                                            cur_task.title = input.value().to_string();
                                        }
                                        input_status = InputStatus::Empty
                                    }
                                    TaskStep::Details => {
                                        if let Some(cur_task) = cur_folder.get_selected_task() {
                                            cur_task.task = input.value().to_string();
                                        }
                                        input_status = InputStatus::Empty
                                    }
                                    TaskStep::Status => {
                                        if let Some(cur_task) = cur_folder.get_selected_task() {
                                            cur_task.status.status = input.value().to_string();
                                        }
                                        input_status =
                                            InputStatus::Request(InputRequestType::EditTask {
                                                step: TaskStep::StatusColor,
                                            });
                                    }
                                    TaskStep::StatusColor => {
                                        if let Some(cur_task) = cur_folder.get_selected_task() {
                                            if let Ok(color) =
                                                input.value().to_string().parse::<u8>()
                                            {
                                                cur_task.status.color = color;
                                            }
                                        }

                                        input_status = InputStatus::Empty
                                    }
                                },
                                InputRequestType::ConfirmDelete => {
                                    if input.value().to_uppercase() == "Y" {
                                        cur_folder.delete_selected();
                                    }
                                    input_status = InputStatus::Empty
                                }
                            }
                            input = input.with_value("".to_string())
                        }
                        _ => {
                            input.handle_event(&Event::Key(key_event));
                        }
                    },
                }
            }
        }
    }
    Ok(())
}

fn render_tabs<B: Backend>(
    frame: &mut Frame<B>,
    chunks: &Chunks,
    selected_tab: usize,
    tabs: Vec<&'static str>,
) {
    let titles = tabs.iter().map(|t| Line::from(*t)).collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .title("R-Tasks")
                .borders(Borders::ALL)
                .border_type(ratatui::widgets::BorderType::Rounded)
                .style(Style::default().fg(Color::White)),
        )
        .select(selected_tab)
        .style(Style::default().fg(Color::Cyan))
        .highlight_style(Style::default().fg(Color::LightGreen));

    frame.render_widget(tabs, chunks.title_bar())
}

fn render_help<B: Backend>(frame: &mut Frame<B>, chunks: &Chunks) {
    let help = Paragraph::new(vec![
        Line::from(" <q> QUIT "),
        Line::from(" <n> NEW "),
        Line::from(" <e> EDIT "),
        Line::from(" <d> DELETE "),
        Line::from(" <w> SAVE "),
    ])
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .title("Controls")
            .border_type(BorderType::Plain),
    );

    frame.render_widget(help, chunks.message_popup());
}

struct Chunks {
    main: Rc<[Rect]>,
    sub: Rc<[Rect]>,
    detail: Rc<[Rect]>,
    input_popup: Rect,
    message_popup: Rect,
}

impl Chunks {
    pub fn title_bar(&self) -> Rect {
        self.main[0]
    }

    pub fn left_menu(&self) -> Rect {
        self.sub[0]
    }

    pub fn right_menu(&self) -> Rect {
        self.sub[1]
    }

    pub fn top_detail(&self) -> Rect {
        self.detail[0]
    }

    pub fn detail(&self) -> Rect {
        self.detail[1]
    }

    pub fn bottom_detail(&self) -> Rect {
        self.detail[2]
    }

    pub fn input_popup(&self) -> Rect {
        self.input_popup
    }

    pub fn message_popup(&self) -> Rect {
        self.message_popup
    }
}

fn make_chunks<T: Backend>(frame: &Frame<T>) -> Chunks {
    let main_chunks = Layout::new()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(10)])
        .split(frame.size());

    let sub_chunks = Layout::new()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(main_chunks[1]);

    let detail_chunks = Layout::new()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Percentage(100),
            Constraint::Min(3),
        ])
        .margin(1)
        .split(sub_chunks[1]);

    let temp_popup = Layout::new()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Min(3),
            Constraint::Percentage(50),
        ])
        .split(frame.size());

    let input_popup = Layout::new()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .split(temp_popup[1])[1];

    let temp_popup = Layout::new()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(100), Constraint::Min(7)])
        .split(frame.size());

    let message_popup = Layout::new()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(100), Constraint::Min(15)])
        .split(temp_popup[1])[1];

    Chunks {
        main: main_chunks,
        sub: sub_chunks,
        detail: detail_chunks,
        input_popup,
        message_popup,
    }
}
