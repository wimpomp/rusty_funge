use std::cmp::{min, max};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread::{spawn, sleep};
use std::time::{Duration, Instant};
use anyhow::{Error, Result};
use cursive::{Cursive, CursiveExt, Printer, Vec2};
use cursive::view::View;
use cursive::theme::{BorderStyle, ColorStyle, Palette, Theme};
use cursive::event::{Event, EventResult, Key};
use cursive::views::{Dialog, EditView};
use rusty_funge::{Int, Funge, join, ord, IO, cast_int, chr, Rect, IP};


#[derive(Clone)]
struct FungeDelta<I: Int> {
    code: HashMap<Vec<isize>, I>,
    ips: Vec<IP<I>>,
    output: usize,
    input: Vec<String>
}

impl<I: Int> FungeDelta<I> {
    fn new(code: HashMap<Vec<isize>, I>, ips: Vec<IP<I>>, output: usize, input: Vec<String>) -> Self {
        Self { code, ips, output, input }
    }
}


#[derive(Clone)]
struct FungeHist<I: Int> {
    maxlen: usize,
    history: Vec<FungeDelta<I>>,
    last: Option<Funge<I>>
}

impl<I: Int> FungeHist<I> {
    fn new() -> Self {
        Self { maxlen: 16348, history: Vec::new(), last: None }
    }

    fn len(&self) -> usize {
        self.history.len()
    }

    fn push(&mut self, old: &Funge<I>, new: &Result<Funge<I>>) {
        if let Ok(new) = new {
            let mut code = HashMap::new();
            for (y, (line_old, line_new)) in old.code.orig_code.iter().zip(new.code.orig_code.iter()).enumerate() {
                if line_old != line_new {
                    for x in 0..min(line_old.len(), line_new.len()) {
                        if line_old[x] != line_new[x] {
                            code.insert(vec![x as isize, y as isize], line_old[x]);
                        }
                    }
                    for x in min(line_old.len(), line_new.len())..max(line_old.len(), line_new.len()) {
                        code.insert(vec![x as isize, y as isize], line_old[x]);
                    }
                }
            }
            for (pos, op) in &new.code.new_code {
                if *op != old.code[pos] {
                    code.insert(pos.to_owned(), *op);
                }
            }
            let ips = old.ips.clone();
            let output = new.output.len() - old.output.len();
            let input = old.input.store.to_owned().into_iter().rev().take(old.input.len() - new.input.len()).rev().collect();
            self.history.push(FungeDelta::new(code, ips, output, input));
            if self.len() > self.maxlen {
                self.history.remove(0);
            }
        } else {
            self.last = Some(old.clone());
        }

    }

    fn pop(&mut self, funge: Result<Funge<I>>) -> Funge<I> {
        match funge {
            Ok(mut funge) => {
                match self.history.pop() {
                    Some(delta) => {
                        for (pos, op) in delta.code {
                            funge.code.insert(pos, op);
                        }
                        funge.ips = delta.ips;
                        for _ in 0..delta.output {
                            funge.output.store.pop();
                        }
                        funge.input.store.extend(delta.input);
                        funge.steps -= 1;
                        funge
                    }
                    None => funge
                }
            }
            _ => self.last.take().expect("There should be a funge here.")
        }
    }
}


struct FungeDebug<I: Int> {
    funge: Option<Result<Funge<I>>>,
    history: FungeHist<I>,
    interval: f64,
    running: bool,
    stop_op: Option<I>
}

impl<I: Int> FungeDebug<I> {
    fn new(funge: Funge<I>) -> Self {
        Self {
            funge: Some(Ok(funge)),
            history: FungeHist::new(),
            interval: 0.05,
            running: false,
            stop_op: None
        }
    }

    fn step_back(&mut self) {
        self.running = false;
        if let Some(new) = self.funge.take() {
            self.funge = Some(Ok(self.history.pop(new)));
        }
    }

    fn step(&mut self) {
        self.funge = match self.funge.take() {
            Some(Ok(funge)) => {
                let old = funge.clone();
                let new = funge.step();
                self.history.push(&old, &new);
                Some(new)
            }
            funge => funge
        }
    }
}


fn input_dialog() -> Result<String> {
    let mut app = Cursive::new();
    app.add_layer(Dialog::new().title("Funge is asking for input").content(EditView::new()));
    app.add_global_callback(Key::Enter, |app| app.quit());
    app.set_theme(Theme { shadow: false, borders: BorderStyle::None, palette: Palette::default() });
    app.run();
    if let Some(view) = app.pop_layer() {
        if let Ok(dialog) = view.downcast::<Dialog>() {
            if let Some(edit) = dialog.get_content().downcast_ref::<EditView>() {
                return Ok(edit.get_content().as_ref().to_string())
            }
        }
    }
    Err(Error::msg("Input went wrong!"))
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
                        None => input_dialog()?,
                        Some(s) => s
                    })
                })).with_output(IO::new()
                .with_output(|store, s| {
                    Ok(store.push(s))
                })))))
        })
    }

    fn step_back(&mut self) {
        if let Ok(mut funge) = self.funge.lock() {
            funge.step_back()
        }
    }

    fn step(&mut self) {
        if let Ok(mut funge) = self.funge.lock() {
            funge.step();
        }
    }

    pub fn step_n(&mut self, n: usize) {
        if let Ok(mut funge) = self.funge.lock() {
            for _ in 0..n {
                funge.step();
            }
        }
    }

    fn new_mutex(&self) -> Self {
        Self { funge: Arc::clone(&self.funge) }
    }

    fn is_running(&self) -> bool {
        match self.funge.lock() {
            Ok(mut funge) => {
                let running = if !funge.running {
                    false
                } else {
                    match funge.funge.as_ref() {
                        Some(Ok(f)) => {
                            if let Some(op) = funge.stop_op {
                                let mut running = true;
                                for pos in f.ips_pos() {
                                    if f.code[&pos] == op {
                                        funge.stop_op = None;
                                        running = false;
                                        break
                                    }
                                }
                                running
                            } else {
                                true
                            }
                        }
                        _ => false
                    }
                };
                if !running {
                    funge.running = false
                }
                running
            }
            _ => false
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
        let mut funge = self.new_mutex();
        { funge.funge.lock().unwrap().running = true; }
        spawn(move || {
            loop {
                let instant = Instant::now();
                funge.step();
                let duration = Duration::from_micros(match funge.funge.lock() {
                    Ok(f) => (f.interval * 1e6) as u64,
                    Err(_) => 100000
                });
                if !funge.is_running() {
                    break
                }
                let elapsed = instant.elapsed();
                if duration > elapsed {
                    sleep(duration - elapsed)
                }
            }
        });
    }

    pub(crate) fn debug(self, interval: Option<f64>) {
        let mut app = Cursive::new();
        if let Some(interval) = interval {
            { self.funge.lock().unwrap().interval = interval; }
            self.toggle_run();
        }
        app.add_layer(self);
        app.add_global_callback(Key::Esc, |app| app.quit());
        app.set_autorefresh(true);
        app.set_theme(Theme { shadow: false, borders: BorderStyle::None, palette: Palette::default() });
        app.run();
    }

    fn wrap(string: String, width: usize) -> Vec<String> {
        let mut lines = Vec::new();
        let mut _a: &str = "";
        for mut line in string.lines() {
            while line.len() > width {
                (_a, line) = line.split_at(width);
                lines.push(_a.to_string());
            }
            if line.len() > 0 {
                lines.push(line.to_string());
            }
        }
        lines
    }
}

impl<I: Int> View for FungeView<I> {
    fn draw(&self, printer: &Printer) {
        if let Ok(funge_mutex) = self.funge.lock().as_ref()  {
            let hist_len = funge_mutex.history.len();
            let running = funge_mutex.running;
            match funge_mutex.funge.as_ref() {
                Some(Ok(funge)) => {
                    let cheight = (printer.size.y / 2) as isize;
                    let cwidth = printer.size.x as isize;
                    let fheight = funge.extent.height();
                    let fwidth = funge.extent.width();
                    let (top, bottom) = if cheight >= fheight {
                        (funge.extent.top, funge.extent.bottom)
                    } else {
                        let y = funge.ips_pos().iter().map(|i| i[1]).sum::<isize>() / (funge.ips.len() as isize);
                        let top = max(y - &cheight / 2, funge.extent.top);
                        (top, top + cheight)
                    };
                    let (left, right) = if cwidth >= fwidth {
                        (funge.extent.left, funge.extent.right)
                    } else {
                        let x = funge.ips_pos().iter().map(|i| i[0]).sum::<isize>() / (funge.ips.len() as isize);
                        let left = max(x - &cwidth / 2, funge.extent.left);
                        (left, left + cwidth)
                    };
                    for (n, line) in funge.code.get_string(Rect::new(left, right, top, bottom)).iter().enumerate() {
                        printer.print((0, n), line);
                    }
                    for pos in funge.ips_pos() {
                        if (left <= pos[0]) & (pos[0] < right) & (top <= pos[1]) & (pos[1] < bottom) {
                            let c = match cast_int::<u8, _>(funge.code[&pos]) {
                                Ok(n @ 32..=126) | Ok(n @ 161..=255) => n,
                                _ => 164
                            };
                            let c = chr(c).expect("c can only be valid u8 for char");
                            printer.with_color(ColorStyle::highlight(),
                                               |printer| {
                                                   printer.print(((pos[0] - left) as usize, (pos[1] - top) as usize), &c.to_string());
                                               }
                            )
                        }
                    }

                    let mut n = (bottom - top) as usize;
                    let offset: Vec<Vec<isize>> = funge.ips.iter().map(|ip| ip.offset.clone()).collect();
                    printer.print((0, n + 1), &format!("top-left: {}, {}, ip pos: {:?}, offset: {:?}",
                                                       top, left, funge.ips_pos(), offset));
                    let cwidth = cwidth as usize;
                    let mut stack = Self::wrap(funge.get_stack_string(), cwidth);
                    let mut output = Self::wrap(funge.output.get(), cwidth);
                    if printer.size.y >= n + 9 {
                        stack = stack.into_iter().rev().take(printer.size.y / 5).rev().collect();
                        if printer.size.y >= stack.len() + n + 9 {
                            output = output.into_iter().rev().take(printer.size.y - stack.len() - n - 9).rev().collect();
                        } else {
                            output = Vec::new();
                        }
                    } else {
                        stack = Vec::new();
                        output = Vec::new();
                    }

                    printer.print((0, n + 3), &format!("stacks:"));
                    n += 4;
                    for line in stack {
                        printer.print((0, n), &*line);
                        n += 1;
                    }
                    printer.print((0, n + 1), "output:");
                    n += 2;
                    for line in output {
                        printer.print((0, n), &*line);
                        n += 1;
                    }
                    printer.print((0, n + 1), &format!("steps: {}", funge.steps));

                    let mut text = vec!["esc: quit"];
                    if hist_len > 0 {
                        text.push("backspace: back");
                    }
                    if running {
                        text.push("space: pause")
                    } else {
                        text.push("space: run")
                    }
                    text.push("enter: step");
                    let interval = format!("interval: {} up/down arrow", funge_mutex.interval);
                    text.push(&*interval);
                    printer.print((0, printer.size.y - 1), &*join(&text, ", "));
                }
                Some(Err(e)) => {
                    printer.print((0, 0), "Error occured:");
                    printer.print((0, 1), &*format!("{}", e));
                    printer.print((0, 2), &*format!("running: {}", running));
                    printer.print((0, 3), "esc: quit, backspace: back");
                }
                None => {}
            }
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
                if interval < 0.001 {
                    funge.interval = 0.001;
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
                if let Ok(op) = ord(c) {
                    self.funge.lock().unwrap().stop_op = Some(op);
                    self.run();
                }
                EventResult::Consumed(None)
            }
            _ => EventResult::Ignored
        }
    }
}