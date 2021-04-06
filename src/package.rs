use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::sync::{Arc, Mutex};

pub struct Package {
    pub name: String,
}

pub struct PackageDownloader {
    packages: Arc<Vec<String>>,
    start_i: usize,
    num_packages: usize,
    package_send: Sender<Event>,
}

impl PackageDownloader {
    pub fn new(packages: Arc<Vec<String>>,
               start_i: usize,
               num_packages: usize,
               package_send: Sender<Event>) -> Self {
        Self {
            packages,
            start_i,
            num_packages,
            package_send,
        }
    }

    fn get_next_package_name(&self,
                             i: usize) -> &String {
        &self.packages[i % self.packages.len()]
    }

    pub fn run(&self,
               pkg_checksum: Arc<Mutex<Checksum>>) {
        // Generate a set of packages and place them into the event queue
        // Update the package checksum with each package name
        for i in 0..self.num_packages {
            let name = self.get_next_package_name(self.start_i + i).clone();

            // TODO: Maybe XOR in each thread locally and then combine later?
            // TODO: Critical section too large
            {
                pkg_checksum
                    .lock()
                    .unwrap()
                    .update(Checksum::with_sha256(&name));
            }

            self.package_send
                .send(Event::DownloadComplete(Package { name }))
                .unwrap();
        }
    }
}
