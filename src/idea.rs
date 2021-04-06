use super::checksum::Checksum;
use super::Event;
use crossbeam::channel::Sender;
use std::sync::{Arc, Mutex};

pub struct Idea {
    pub name: String,
    pub num_packages: usize,
}

pub struct IdeaGenerator {
    ideas: Arc<Vec<(String, String)>>,
    start_i: usize,
    num_ideas: usize,
    num_students: usize,
    num_packages: usize,
    idea_send: Sender<Event>,
    packages_per_idea: usize,
    extra_packages: usize,
    idea_checksum: Checksum,
}

impl IdeaGenerator {
    pub fn new(ideas: Arc<Vec<(String, String)>>,
               start_i: usize,
               num_ideas: usize,
               num_students: usize,
               num_packages: usize,
               idea_send: Sender<Event>) -> Self {
        assert_ne!(num_ideas, 0);
        return Self {
            ideas,
            start_i,
            num_ideas,
            num_students,
            num_packages,
            idea_send,
            packages_per_idea: num_packages / num_ideas,
            extra_packages: num_packages % num_ideas,
            idea_checksum: Checksum::default(),
        }
    }

    // Idea names are generated from cross products between product names and customer names. The
    // index wraps around once it reaches the number of tuples in the products vs customers cross
    // product.
    fn get_next_idea_name(&self, i: usize) -> String {
        let pair = &self.ideas[i % self.ideas.len()];
        return format!("{} for {}", pair.0, pair.1);
    }

    pub fn run(&mut self) -> Checksum {
        // Generate a set of new ideas and place them into the event-queue
        // Update the idea checksum with all generated idea names
        for i in 0..self.num_ideas {
            let name = self.get_next_idea_name(self.start_i + i);
            let num_packages = self.packages_per_idea + (i < self.extra_packages) as usize;
            let idea = Idea {
                name,
                num_packages,
            };

            // Update checksum locally
            self.idea_checksum.update(Checksum::with_sha256(&idea.name));

            self.idea_send.send(Event::NewIdea(idea)).unwrap();
        }

        // Push student termination events into the event queue.
        //
        // It seems like each idea generator thread pushes num_students poison pills so that there
        // are guaranteed to be enough for all students no matter how many idea generators there
        // are.
        for _ in 0..self.num_students {
            self.idea_send.send(Event::OutOfIdeas).unwrap();
        }

        self.idea_checksum.clone()
    }
}