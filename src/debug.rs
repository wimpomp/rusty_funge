use std::cmp::{min, max};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;
use cursive::{Cursive, CursiveExt, Vec2};
use cursive::view::View;
use cursive::Printer;
use cursive::event::{Event, EventResult};
use cursive::theme::ColorStyle;
use crate::Funge;


struct FungeMutex {
    funge: Option<Funge>
}

impl FungeMutex {
    fn new(funge: Funge) -> Self {
        Self { funge: Some(funge) }
    }

    fn step(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut funge = self.funge.to_owned().ok_or("No funge found")?;
        if !funge.terminated {
            funge = funge.step()?;
        }
        let terminated = funge.terminated;
        self.funge = Some(funge);
        Ok(terminated)
    }

    fn funge_ref(&self) -> Option<&Funge> {
        self.funge.as_ref()
    }
}

impl Display for FungeMutex {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.funge.as_ref().expect("No funge found"))
    }
}


pub(crate) struct FungeView {
    funge: Arc<Mutex<FungeMutex>>
}

impl FungeView {
    pub (crate) fn new(funge: Funge) -> Result<Self, Box<dyn Error>> {
        Ok(FungeView { funge: Arc::new(Mutex::new(FungeMutex::new(funge.with_output()?))) } )
    }

    fn step(&mut self) -> Result<bool, Box<dyn Error>> {
        self.funge.lock().unwrap().step()
    }

    fn get_mutex(&self) -> Self {
        Self { funge: Arc::clone(&self.funge) }
    }

    pub(crate) fn debug(self, interval: Option<f64>) -> Result<(), Box<dyn Error>> {
        let mut app = Cursive::new();
        match interval {
            None => {}
            Some(interval) => {
                let duration = Duration::from_micros((interval * 1e6) as u64);
                let mut funge_mutex = self.get_mutex();
                app.set_fps(max((1f64 / interval) as u32, 50));
                spawn(move || {
                    loop {
                        sleep(duration);
                        match funge_mutex.step() {
                            Ok(terminated) => if terminated { break },
                            Err(_) => break
                        }
                    }
                });
            }
        }
        app.add_layer(self);
        app.add_global_callback('q', |app| app.quit());
        app.run();
        Ok(())
    }
}

impl View for FungeView {
    fn draw(&self, printer: &Printer) {
        let (text, ips_pos, terminated) = {
            let lock = self.funge.lock();
            let funge = lock.as_ref().unwrap().funge_ref().unwrap();
            (format!("{}", funge), funge.ips_pos(), funge.terminated)
        };
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            printer.print((0, i), line);
        }
        for pos in ips_pos.iter() {
            if (pos[0] >= 0) & (pos[1] >= 0) {
                let x = pos[0] as usize;
                let y = pos[1] as usize + 1;
                printer.with_color(ColorStyle::highlight(), |printer| printer.print((x, y), &*lines[y].chars().nth(x).unwrap().to_string()));
            }
        }
        let mut bottom = String::from("Press 'q' to quit");
        if terminated {
            bottom.push_str(".");
        } else {
            bottom.push_str(", any other key to continue.");
        }
        printer.print((0, lines.len() + 1), &*bottom);
    }

    fn required_size(&mut self, constraint: Vec2) -> Vec2 {
        let text = format!("{}", self.funge.lock().as_ref().unwrap());
        let x = match text.lines().map(|line| line.len()).collect::<Vec<usize>>().iter().max() {
            None => 0,
            Some(x) => *x
        };
        Vec2::new(min(max(80, x), constraint.x), min(max(25, text.lines().count() + 2), constraint.y))
    }

    fn on_event(&mut self, event: Event) -> EventResult {
        match event {
            Event::Char('q') => EventResult::Ignored,
            Event::Char(_) => {
                self.step().ok();
                EventResult::Consumed(None)
            }
            Event::Key(_) => {
                self.step().ok();
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored
        }
    }
}