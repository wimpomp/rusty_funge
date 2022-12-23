use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};
use std::thread::{spawn, sleep};
use std::time::{Duration, Instant};
use cursive::{Cursive, CursiveExt, Printer, Vec2};
use cursive::view::View;
use cursive::theme::{BorderStyle, ColorStyle, Palette, Theme};
use cursive::event::{Event, EventResult, Key};
use rusty_funge::{Int, Funge, join, ord, IO};
use anyhow::Result;


// TODO: Use anyhow::Result
#[derive(Clone)]
enum FungeError<I: Int> {
    Funge(Funge<I>),
    Error(String)
}


struct FungeDebug<I: Int> {
    funge: Option<FungeError<I>>,
    history: Vec<Funge<I>>,
    interval: f64,
    running: bool,
    stop_op: Option<I>
}

impl<I: Int> FungeDebug<I> {
    fn new(funge: Funge<I>) -> Self {
        Self {
            funge: Some(FungeError::Funge(funge)),
            history: Vec::new(),
            interval: 0.05,
            running: false,
            stop_op: None
        }
    }

    fn step_back(&mut self) {
        match self.history.pop() {
            Some(funge) => {
                self.running = false;
                self.funge = Some(FungeError::Funge(funge));
            }
            None => {}
        }
    }

    fn is_terminated(&self) -> bool {
        match &self.funge {
            Some(FungeError::Funge(f)) if !f.terminated => false,
            _ => true
        }
    }

    fn step(&mut self) {
        match self.funge.to_owned() {
            Some(FungeError::Funge(funge)) if !funge.terminated => {
                self.history.push(funge.clone());
                if self.history.len() > 16384 {
                    self.history.remove(0);
                }
                match funge.step() {
                    Ok(f) => self.funge = Some(FungeError::Funge(f)),
                    Err(e) => {
                        self.funge = Some(FungeError::Error(e.to_string()))
                    }
                }
            }
            Some(f) => self.funge = Some(f),
            None => {}
        }
    }
}

impl<I: Int> Display for FungeDebug<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self.funge.as_ref().expect("No funge found") {
            FungeError::Funge(funge) => write!(f, "{}", funge),
            FungeError::Error(e) => write!(f, "{}", e)
        }
    }
}


pub(crate) struct FungeView<I: Int> {
    funge: Arc<Mutex<FungeDebug<I>>>
}

impl<I: Int> FungeView<I> {
    pub (crate) fn new(funge: Funge<I>, input: Vec<String>) -> Result<Self> {
        Ok(FungeView { funge: Arc::new(Mutex::new(FungeDebug::new(
            funge.with_input(IO::new()
                .with_store(input)
                .with_input(|store| {
                    Ok(match store.pop() {
                        None => String::from("5"),  // TODO: cursive input
                        Some(s) => s
                    })
                })).with_output(IO::new()
                .with_output(|store, s| {
                    Ok(store.push(s))
                })))))
        })
    }

    fn step_back(&mut self) {
        match self.funge.lock() {
            Ok(mut f) => f.step_back(),
            Err(_) => {}
        }
    }

    fn step(&mut self) {
        match self.funge.lock() {
            Ok(mut f) => f.step(),
            Err(_) => {}
        }
    }

    pub fn step_n(&mut self, n: usize) {
        match self.funge.lock() {
            Ok(mut funge) => {
                for _ in 0..n {
                    funge.step();
                    if funge.is_terminated() { break }
                }
            }
            Err(_) => {}
        }
    }

    fn new_mutex(&self) -> Self {
        Self { funge: Arc::clone(&self.funge) }
    }

    fn is_running(&self) -> bool {
        match self.funge.lock() {
            Ok(mut funge) => {
                if funge.is_terminated() | !funge.running { return false }
                if let Some(op) = funge.stop_op {
                    match funge.funge.as_ref() {
                        Some(FungeError::Funge(f)) => {
                            for pos in f.ips_pos() {
                                if f.code[&pos] == op {
                                    funge.stop_op = None;
                                    funge.running = false;
                                    return false
                                }
                            }
                        }
                        Some(FungeError::Error(_)) => {
                            funge.running = false;
                            return false
                        }
                        None => return false
                    }
                }
                true
            }
            Err(_) => false
        }
    }

    fn toggle_run(&self) {
        let running = { self.funge.lock().unwrap().running };
        match running {
            true => self.pause(),
            false => self.run()
        }
    }

    fn pause(&self) {
        self.funge.lock().unwrap().running = false;
    }

    fn run(&self) {
        let mut funge_mutex = self.new_mutex();
        { funge_mutex.funge.lock().unwrap().running = true; }
        spawn(move || {
            loop {
                let instant = Instant::now();
                funge_mutex.step();
                let duration = Duration::from_micros(match funge_mutex.funge.lock() {
                    Ok(f) => (f.interval * 1e6) as u64,
                    Err(_) => 100000
                });
                if !funge_mutex.is_running() {
                    funge_mutex.funge.lock().unwrap().running = false;
                    break
                }
                let sleep_time = duration - instant.elapsed();
                if sleep_time.as_secs_f64() > 0f64 { sleep(duration) }
            }
        });
    }

    pub(crate) fn debug(self, interval: Option<f64>) {
        let mut app = Cursive::new();
        match interval {
            None => {}
            Some(interval) => {
                { self.funge.lock().unwrap().interval = interval; }
                self.toggle_run();
            }
        }
        app.add_layer(self);
        app.add_global_callback(Key::Esc, |app| app.quit());
        app.set_autorefresh(true);
        app.set_theme(Theme { shadow: false, borders: BorderStyle::None, palette: Palette::default() });
        app.run();
    }
}

impl<I: Int> View for FungeView<I> {
    fn draw(&self, printer: &Printer) {
        match self.funge.lock().as_ref() {
            Ok(funge_mutex) => {
                let hist_len = funge_mutex.history.len();
                let running = funge_mutex.running;
                match funge_mutex.funge.as_ref() {
                    Some(FungeError::Funge(funge)) => {
                        let text = format!("{}", funge);
                        let lines: Vec<&str> = text.lines().collect();
                        for (i, line) in lines.iter().enumerate() {
                            printer.print((0, i), line);
                        }
                        for pos in funge.ips_pos().iter() {
                            if (pos[0] >= 0) & (pos[1] >= 0) {
                                let x = pos[0] as usize;
                                let y = pos[1] as usize + 1;
                                printer.with_color(ColorStyle::highlight(),
                                                   |printer| {
                                                       match lines[y].chars().nth(x) {
                                                           None => {}
                                                           Some(l) => printer.print((x, y), &*l.to_string())
                                                       }
                                                   });
                            }
                        }
                        let n = lines.len() + 1;

                        let mut text = vec!["esc: quit"];
                        if hist_len > 0 {
                            text.push("backspace: back");
                        }
                        if running {
                            text.push("space: pause")
                        } else {
                            text.push("space: run")
                        }
                        if !funge.terminated {
                            text.push("enter: step")
                        }
                        let interval = format!("interval: {} up/down arrow", funge_mutex.interval);
                        text.push(&*interval);
                        printer.print((0, n + 1), &*join(&text, ", "));
                    }
                    Some(FungeError::Error(e)) => {
                        printer.print((0, 0), "Error occured:");
                        printer.print((0, 1), &*format!("{}", e));
                        printer.print((0, 3), "esc: quit, backspace: back");
                    }
                    None => {}
                }
            }
            Err(_) => {}
        }
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        constraint
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Key(Key::Esc) => EventResult::Ignored,
            Event::Key(Key::Backspace) => {
                self.step_back();
                EventResult::Consumed(None)
            }
            Event::Char(' ') => {
                self.toggle_run();
                EventResult::Consumed(None)
            }
            Event::Key(Key::Enter) => {
                self.step();
                EventResult::Consumed(None)
            }
            Event::Key(Key::Up) => {
                let lock = self.funge.lock();
                let mut funge = lock.unwrap();
                let interval = funge.interval / 2.0;
                if interval < 0.01 {
                    funge.interval = 0.01;
                } else {
                    funge.interval = interval;
                }
                EventResult::Consumed(None)
            }
            Event::Key(Key::Down) => {
                self.funge.lock().unwrap().interval *= 2.0;
                EventResult::Consumed(None)
            }
            Event::Char(c) => {
                match ord(c) {
                    Ok(i) => self.funge.lock().unwrap().stop_op = Some(i),
                    Err(_) => {}
                }
                self.run();
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored
        }
    }
}