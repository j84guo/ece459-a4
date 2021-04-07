use std::io::Write;
use std::io;

use crossbeam::channel::{Receiver, Sender};

use super::{checksum::Checksum, Event, idea::Idea, package::Package};

pub struct Student {
    id: usize,
    idea_recv: Receiver<Option<Idea>>,
    package_recv: Receiver<Package>,
    idea_checksum: Checksum,
    package_checksum: Checksum,
    done_message: String,
}

impl Student {
    pub fn new(id: usize,
               idea_recv: Receiver<Option<Idea>>,
               package_recv: Receiver<Package>) -> Self {
        Self {
            id,
            idea_recv,
            package_recv,
            idea_checksum: Checksum::default(),
            package_checksum: Checksum::default(),
            done_message: String::new(),
        }
    }

    fn build_idea(&mut self, idea: Idea, packages: &Vec<Package>) {
        // Update idea and package checksums
        // All of the packages used in the update are deleted, along with the idea
        self.idea_checksum.update(Checksum::with_sha256(&idea.name));
        for package in packages {
            self.package_checksum.update(Checksum::with_sha256(&package.name));
        }

        // TODO: Can this be made faster somehow?
        // We want the subsequent prints to be together, so we lock stdout
        let mut s = format!("\nStudent {} built {} using {} packages\nIdea checksum: {}\nPackage checksum: {}\n",
                            self.id, idea.name, packages.len(), self.idea_checksum, self.package_checksum);
        for package in packages {
            s += &format!("> {}\n", package.name);
        }
        self.done_message += &s;
    }

    pub fn run(&mut self) -> (Checksum, Checksum) {
        let mut packages = Vec::<Package>::new();
        loop {
            match self.idea_recv.recv().unwrap() {
                Some(idea) => {
                    for _ in 0..idea.num_packages {
                        packages.push(self.package_recv.recv().unwrap());
                    }
                    self.build_idea(idea, &packages);
                    packages.clear();
                }
                None => {
                    write!(io::stdout(), "{}", self.done_message).unwrap();
                    return (self.idea_checksum.clone(), self.package_checksum.clone())
                }
            }
        }
    }
}
