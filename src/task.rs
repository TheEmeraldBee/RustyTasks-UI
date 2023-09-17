use std::{collections::VecDeque, fs};

use ratatui::{
    style::Style,
    widgets::{List, ListItem, Paragraph},
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default)]
pub enum TaskFile {
    #[default]
    Main,
    Trash,
    Complete,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Status {
    pub status: String,
    pub color: u8,
}

impl Default for Status {
    fn default() -> Self {
        Self {
            status: String::from("Incomplete"),
            color: 5,
        }
    }
}

impl Status {
    pub fn to_paragraph(&self) -> Paragraph {
        Paragraph::new(self.status.clone())
            .style(Style::new().fg(ratatui::style::Color::Indexed(self.color)))
    }
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct Task {
    pub title: String,
    pub task: String,
    pub status: Status,
}

#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Folder {
    pub name: String,
    tasks: Vec<Task>,
    folders: Vec<Folder>,
    #[serde(skip_serializing, default)]
    selected: usize,
}

impl Folder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn new_task(&mut self, task: Task) -> &mut Task {
        self.tasks.push(task);

        let task_len = self.tasks.len() - 1;

        self.tasks.get_mut(task_len).expect("Task should exist")
    }

    pub fn new_folder(&mut self, name: String) -> &mut Folder {
        let mut folder = Folder::new();
        folder.name = name;
        self.folders.push(folder);

        let folder_len = self.folders.len() - 1;

        self.folders
            .get_mut(folder_len)
            .expect("Folder should exist")
    }

    pub fn read_or_create() -> anyhow::Result<Self> {
        if let Some(dirs) = directories::UserDirs::new() {
            let home_dir = dirs.home_dir();

            let tasks_dir = home_dir.join(".rtasks");

            // Ensure the directory exists
            #[allow(clippy::single_match)]
            match fs::create_dir(tasks_dir.clone()).is_ok() {
                true => {}
                false => {}
            }

            // Try to read the file
            if let Ok(data) = fs::read_to_string(tasks_dir.join("tasks.json")) {
                Ok(serde_json::from_str(&data)?)
            } else {
                // The file doesn't exist, create it
                let folder = Folder::default();

                let folder_json = serde_json::to_string_pretty(&folder)?;

                fs::write(tasks_dir.join("tasks.json"), folder_json)?;

                Ok(folder)
            }
        } else {
            Err(anyhow!("Failed to find user home directory"))
        }
    }

    // Write this to the specified file
    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(dirs) = directories::UserDirs::new() {
            let home_dir = dirs.home_dir();

            let tasks_dir = home_dir.join(".rtasks");

            fs::write(
                tasks_dir.join("tasks.json"),
                serde_json::to_string_pretty(&self)?,
            )?;
        }
        Ok(())
    }

    pub fn delete_selected(&mut self) {
        if self.folders.is_empty() && self.tasks.is_empty() {
            return;
        }

        if self.selected < self.folders.len() {
            self.folders.remove(self.selected);
            if self.selected > 0 {
                self.selected -= 1;
            }
        } else {
            self.tasks.remove(self.selected - self.folders.len());
            if self.selected > 0 {
                self.selected -= 1;
            }
        }
    }

    pub fn get_folder(&mut self, path: impl Into<VecDeque<String>>) -> anyhow::Result<&mut Folder> {
        let mut path = path.into();
        if let Some(item) = path.pop_front() {
            for folder in &mut self.folders {
                if folder.name == item {
                    return folder.get_folder(path);
                }
            }
            Err(anyhow!("Folder with name {} doesn't exist", item))
        } else {
            Ok(self)
        }
    }

    pub fn get_selected_task(&mut self) -> Option<&mut Task> {
        if self.folders.is_empty() && self.tasks.is_empty() {
            return None;
        }

        if self.selected < self.folders.len() {
            None
        } else if let Some(task) = self.tasks.get_mut(self.selected - self.folders.len()) {
            Some(task)
        } else {
            None
        }
    }

    pub fn adjust_selected(&mut self, dist: i32) {
        let max = self.tasks.len().max(0) as i32 + self.folders.len().max(0) as i32 - 1;

        self.selected = (self.selected as i32 + dist).clamp(0, max).unsigned_abs() as usize;
    }

    pub fn get_selected_folder(&mut self) -> Option<&mut Folder> {
        if self.selected >= self.folders.len() {
            return None;
        }

        Some(&mut self.folders[self.selected])
    }

    pub fn as_list_widget(&mut self) -> List {
        let mut list = vec![];
        // Add the folders to the list
        for folder in &self.folders {
            let style = if list.len() == self.selected {
                Style::default()
                    .fg(ratatui::style::Color::LightCyan)
                    .bg(ratatui::style::Color::DarkGray)
            } else {
                Style::default().fg(ratatui::style::Color::LightCyan)
            };

            list.push(ListItem::new(folder.name.clone()).style(style));
        }

        // Add the tasks to the list
        for task in &self.tasks {
            let style = if list.len() == self.selected {
                Style::default()
                    .fg(ratatui::style::Color::LightGreen)
                    .bg(ratatui::style::Color::DarkGray)
            } else {
                Style::default().fg(ratatui::style::Color::LightGreen)
            };

            list.push(ListItem::new(task.title.clone()).style(style));
        }

        List::new(list)
    }
}
