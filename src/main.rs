#![warn(clippy::all)]
use lab4::{
    checksum::Checksum,
    idea::IdeaGenerator,
    package::PackageDownloader,
    student::Student,
    Event
};
// TODO: Maybe bounded?
use crossbeam::channel::{unbounded, Receiver, Sender};
use std::{env, io};
use std::error::Error;
use std::sync::{Arc, Mutex};
use std::thread::spawn;
use std::time::Duration;
use std::path::Path;
use std::fs::File;
use std::io::BufRead;

#[derive(Debug)]
struct Args {
    pub num_ideas: usize,
    pub num_idea_generators: usize,
    pub num_packages: usize,
    pub num_package_downloaders: usize,
    pub num_students: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<String>>();
    let num_ideas = args.get(1)
        .map_or(Ok(80), |a| a.parse::<usize>())?;
    let num_idea_generators = args.get(2)
        .map_or(Ok(2), |a| a.parse::<usize>())?;
    let num_packages = args.get(3)
        .map_or(Ok(4000), |a| a.parse::<usize>())?;
    let num_package_downloaders = args.get(4)
        .map_or(Ok(6), |a| a.parse::<usize>())?;
    let num_students = args.get(5)
        .map_or(Ok(6), |a| a.parse::<usize>())?;

    let args = Args {
        num_ideas,
        num_idea_generators,
        num_packages,
        num_package_downloaders,
        num_students,
    };
    hackathon(&args);
    Ok(())
}

fn read_lines<P: AsRef<Path>>(path: P) -> io::Result<Vec<String>> {
    let file = File::open(path)?;
    let mut lines = Vec::<String>::new();
    for res in io::BufReader::new(file).lines() {
        lines.push(res?);
    }
    Ok(lines)
}

fn read_ideas() -> io::Result<Vec<(String, String)>> {
    let products = read_lines("data/ideas-products.txt")?;
    let customers = read_lines("data/ideas-customers.txt")?;
    let ideas = products.iter()
        .flat_map(|p| customers.iter()
            .map(move |c| (p.clone(), c.clone())))
        .collect();
    Ok(ideas)
}

fn per_thread_amount(thread_idx: usize, total: usize, threads: usize) -> usize {
    let per_thread = total / threads;
    let extras = total % threads;
    per_thread + (thread_idx < extras) as usize
}

// TODO: Use separate channels for packages, ideas
fn hackathon(args: &Args) -> Result<(), Box<dyn Error>> {
    // Use message-passing channel as event queue
    let (send, recv) = unbounded::<Event>();

    // Checksums of all the generated ideas and packages
    let mut idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut package_checksum = Arc::new(Mutex::new(Checksum::default()));

    // Checksums of the ideas and packages used by students to build ideas. Should match the
    // previous checksums.
    let mut student_idea_checksum = Arc::new(Mutex::new(Checksum::default()));
    let mut student_package_checksum = Arc::new(Mutex::new(Checksum::default()));

    // Store all spawned threads.
    let mut threads = vec![];

    // Spawn num_students student threads.
    for i in 0..args.num_students {
        Sender::clone(&send);
        let mut student = Student::new(i, send.clone(), recv.clone());
        let student_idea_checksum = student_idea_checksum.clone();
        let student_pkg_checksum = student_package_checksum.clone();
        let thread = spawn(move || student.run(student_idea_checksum, student_pkg_checksum));
        threads.push(thread);
    }

    // Spawn num_pkg_gen package downloader threads. Packages are distributed evenly across threads.
    let packages = Arc::new(read_lines("data/packages.txt")?);
    let mut start_i = 0;
    for i in 0..args.num_package_downloaders {
        let num_packages = per_thread_amount(i, args.num_packages, args.num_package_downloaders);
        let downloader = PackageDownloader::new(packages.clone(), start_i, num_packages, send.clone());
        start_i += num_packages;
        let package_checksum = Arc::clone(&package_checksum);

        let thread = spawn(move || downloader.run(package_checksum));
        threads.push(thread);
    }
    assert_eq!(start_i, args.num_packages);

    // Spawn num_idea_gen idea generator threads. Ideas and packages are distributed evenly across threads. In
    // each thread, packages are distributed evenly across ideas.
    // Share between idea generators
    let ideas = Arc::new(read_ideas()?);
    let mut start_i = 0;
    for i in 0..args.num_idea_generators {
        let num_ideas = per_thread_amount(i, args.num_ideas, args.num_idea_generators);
        let num_packages = per_thread_amount(i, args.num_packages, args.num_idea_generators);
        let num_students = per_thread_amount(i, args.num_students, args.num_idea_generators);
        let generator = IdeaGenerator::new(
            ideas.clone(),
            start_i,
            num_ideas,
            num_students,
            num_packages,
            send.clone(),
        );
        let idea_checksum = idea_checksum.clone();
        start_i += num_ideas;

        let thread = spawn(move || generator.run(idea_checksum));
        threads.push(thread);
    }
    assert_eq!(start_i, args.num_ideas);

    // Join all threads
    threads.into_iter().for_each(|t| t.join().unwrap());

    let idea = Arc::get_mut(&mut idea_checksum).unwrap().get_mut().unwrap();
    let student_idea = Arc::get_mut(&mut student_idea_checksum)
        .unwrap()
        .get_mut()
        .unwrap();
    let pkg = Arc::get_mut(&mut package_checksum).unwrap().get_mut().unwrap();
    let student_pkg = Arc::get_mut(&mut student_package_checksum)
        .unwrap()
        .get_mut()
        .unwrap();

    println!("Global checksums:\nIdea Generator: {}\nStudent Idea: {}\nPackage Downloader: {}\nStudent Package: {}",
        idea, student_idea, pkg, student_pkg);

    Ok(())
}
