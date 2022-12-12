use std::collections::HashMap;
use std::{fmt, fs};
use std::fmt::{Debug, Display, Formatter};
use std::io::stdin;
use std::ops::{Index, IndexMut};
use std::error::Error;
use std::path::Path;
use chrono::{Datelike, Timelike};
use rand::Rng;
use chrono::offset::Local;


const VERSION: &str = env!("CARGO_PKG_VERSION");


fn join<T: ToString>(v: &Vec<T>, s: &str) -> String {
    let mut string = String::new();
    if v.len() > 1 {
        for i in 0..v.len() - 1 {
            string.push_str(&v[i].to_string());
            string.push_str(&s);
        }
    }
    if v.len() > 0 {
        string.push_str(&v[v.len() - 1].to_string());
    }
    string
}

fn ord(c: char) -> Result<isize, Box<dyn Error>> {
    Ok(u32::try_from(c)?.try_into()?)
}

fn chr(i: isize) -> Result<char, Box<dyn Error>> {
    Ok(u32::try_from(i)?.try_into()?)
}

fn add(a: &Vec<isize>, b: &Vec<isize>) -> Vec<isize> {
    a.iter().zip(b.iter()).map(|(&a, &b)| a + b).collect()
}

fn sub(a: &Vec<isize>, b: &Vec<isize>) -> Vec<isize> {
    a.iter().zip(b.iter()).map(|(&a, &b)| a - b).collect()
}


#[derive(Clone)]
enum InputEnum {
    StdIn,
    Vector(Vec<String>)
}

#[derive(Clone)]
struct Input {
    source: InputEnum
}

impl Input {
    fn get(&mut self) -> Result<String, Box<dyn Error>> {
        Ok(match self.source {
            InputEnum::StdIn => {
                let mut s = String::new();
                stdin().read_line(&mut s)?;
                s
            }
            InputEnum::Vector(ref mut v) => v.pop().ok_or("No more input!")?
        })
    }
}


#[derive(Clone)]
enum OutputEnum {
    StdOut,
    Vector(Vec<String>)
}

#[derive(Clone)]
struct Output {
    sink: OutputEnum
}

impl Output {
    fn print(&mut self, string: String) {
        match self.sink {
            OutputEnum::StdOut => println!("{}", string),
            OutputEnum::Vector(ref mut v) => v.push(string)
        }
    }
}


#[derive(Clone)]
struct Stack {
    stack: Vec<isize>
}


impl Stack {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn pop(&mut self) -> isize {
        match self.stack.pop() {
            Some(value) => { value }
            None => { 0 }
        }
    }

    fn push(&mut self, cell: isize) {
        self.stack.push(cell)
    }

    fn len(&self) -> usize { self.stack.len() }
}

impl Display for Stack {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}]", join(&self.stack, ", "))
    }
}

impl Index<usize> for Stack {
    type Output = isize;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stack[index]
    }
}

#[derive(Clone)]
struct StackStack {
    stackstack: Vec<Stack>
}

impl StackStack {
    fn new() -> Self {
        Self { stackstack: vec![Stack::new()] }
    }

    fn check_stack(&mut self) {
        if self.is_empty() {
            self.push_stack(Stack::new());
        }
    }

    fn pop(&mut self) -> isize {
        self.check_stack();
        let x = self.len_stack();
        self.stackstack[x - 1].pop()
    }

    fn push(&mut self, cell: isize) {
        self.check_stack();
        let x = self.len_stack();
        self.stackstack[x - 1].push(cell);
    }

    fn pop_stack(&mut self) -> Stack {
        match self.stackstack.pop() {
            Some(stack) => { stack }
            None => { Stack::new() }
        }
    }

    fn push_stack(&mut self, stack: Stack) {
        self.stackstack.push(stack)
    }

    fn len_stack(&self) -> usize { self.stackstack.len() }

    fn len(&self) -> usize {
        if self.len_stack() == 0 {
            0
        } else {
            self.stackstack[self.len_stack() - 1].len()
        }
    }

    fn is_empty(&self) -> bool {
        self.stackstack.is_empty()
    }

    fn clear(&mut self) {
        let l = self.len_stack();
        if l > 0 {
            self.pop_stack();
        }
        self.push_stack(Stack::new());
    }
}

impl Index<usize> for StackStack {
    type Output = Stack;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stackstack[index]
    }
}

impl IndexMut<usize> for StackStack {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.stackstack[index]
    }
}

impl Display for StackStack {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", join(&self.stackstack, "\n"))
    }
}


#[derive(Clone)]
struct IP {
    id: usize,
    position: Vec<isize>,
    delta: Vec<isize>,
    offset: Vec<isize>,
    string: bool,
    stack: StackStack,
    fingerprint_ops: HashMap<isize, ()>,
}


impl IP {
    fn new(funge: &Funge) -> Result<Self, Box<dyn Error>> {
        let mut new = IP {
            id: funge.ips.len(),
            position: vec![0, 0],
            delta: vec![1, 0],
            offset: vec![0, 0],
            string: false,
            stack: StackStack::new(),
            fingerprint_ops: HashMap::new()
        };
        if (new.op(funge) == ord(' ')?) | (new.op(funge) == ord(';')?) {
            new.advance(funge);
        };
        Ok(new)
    }

    fn copy(&self, funge: &Funge) -> Self {
        Self {
            id: funge.ips.len(),
            position: self.position.to_owned(),
            delta: self.delta.to_owned(),
            offset: self.offset.to_owned(),
            string: self.string,
            stack: self.stack.to_owned(),
            fingerprint_ops: self.fingerprint_ops.to_owned()
        }
    }

    fn op(&self, funge: &Funge) -> isize {
        if funge.code.contains_key(&self.position) {
            funge.code[&self.position]
        } else {
            32
        }
    }

    fn reverse(&mut self) {
        self.delta = self.delta.iter().map(|i| -i).collect();
    }

    fn turn_right(&mut self) {
        self.delta = vec![-self.delta[1], self.delta[0]];
    }

    fn turn_left(&mut self) {
        self.delta = vec![self.delta[1], -self.delta[0]];
    }

    fn advance(&mut self, funge: &Funge) {
        if self.string {
            if funge.code[&self.position] == 32 {
                while funge.code[&self.position] == 32 {
                    self.movep(funge)
                }
            } else {
                self.movep(funge)
            }
        } else {
            loop {
                if self.op(funge) != 59 {
                    self.movep(funge)
                }
                if self.op(funge) == 59 {
                    self.movep(funge);
                    while self.op(funge) == 59 {
                        self.movep(funge)
                    }
                    self.movep(funge)
                }
                while self.op(funge) == 32 {
                    self.movep(funge)
                }
                if self.op(funge) != 59 {
                    break;
                }
            }
        }
    }

    fn movep(&mut self, funge: &Funge) {
        self.position = self.next_pos(funge);
    }

    fn check_pos(&self, pos: &Vec<isize>, funge: &Funge) -> bool {
        (funge.extent[0] <= pos[0]) & (pos[0] < funge.extent[1]) &
            (funge.extent[2] <= pos[1]) & (pos[1] < funge.extent[3])
    }

    fn next_pos(&self, funge: &Funge) -> Vec<isize> {
        let mut pos= add(&self.position, &self.delta);
        if !self.check_pos(&pos, funge) {
            loop {
                pos = sub(&pos, &self.delta);
                if !self.check_pos(&pos, funge) {
                    pos = add(&pos, &self.delta);
                    break
                }
            }
        }
        pos
    }

    fn read_string(&mut self) -> Result<String, Box<dyn Error>> {
        let mut string = String::new();
        loop {
            let f = self.stack.pop();
            if f == 0 {
                return Ok(string)
            } else {
                string.push_str(&chr(f)?.to_string())
            }
        }
    }

    fn get_info(&self, funge: &Funge, n: isize) -> Result<Vec<isize>, Box<dyn Error>> {
        let time = Local::now();
        Ok(match n {
            1 => { vec![15] }
            2 => { vec![isize::BITS as isize] }
            3 => {
                let mut f = 0;
                for (i, c) in "wpfunge".chars().enumerate() {
                    f += (256 as isize).pow(i as u32) * ord(c)?;
                }
                vec![f]
            }
            4 => { vec![VERSION.replace(".", "").parse()?] }
            5 => { vec![1] }
            6 => { vec![ord(std::path::MAIN_SEPARATOR)?] }
            7 => { vec![2] }
            8 => { vec![self.id as isize] }
            9 => { vec![0] }
            10 => { self.position.to_owned() }
            11 => { self.delta.to_owned() }
            12 => { self.offset.to_owned() }
            13 => { funge.extent.chunks(2).map(|i| i[0]).collect() }
            14 => { funge.extent.chunks(2).map(|i| i[1]).collect() }
            15 => { vec![((time.year() as isize) - 1900) * 256 * 256 + (time.month() as isize) * 256 + (time.day() as isize)] }
            16 => { vec![(time.hour() as isize) * 256 * 256 + (time.minute() as isize) * 256 + (time.second() as isize)] }
            17 => { vec![self.stack.len_stack() as isize] }
            18 => {
                let mut l = Vec::new();
                for stack in &self.stack.stackstack {
                    l.push(stack.len() as isize);
                }
                l.reverse();
                l
            }
            19 => {
                let mut r = Vec::new();
                let mut args = std::env::args();
                if args.len() > 2 {
                    for i in 2..args.len() {
                        let j: Vec<isize> = args.nth(i).expect("We checked the length.")
                            .chars().map(|i| ord(i).expect("")).collect();
                        r.extend(j);
                        r.push(0);
                    }
                }
                r.push(0);
                let file = args.nth(1).expect("We checked the length.");
                let path = Path::new(&file);
                let j: Vec<isize> = path.file_name().ok_or("No file name.")?
                    .to_str().ok_or("Cannot convert String.")?
                    .chars().map(|i| ord(i).expect("")).collect();
                r.extend(j);
                r.push(0);
                r.push(0);
                r.reverse();
                r
            }
            20 => {
                let mut r = Vec::new();
                let vars = std::env::vars();
                for (key, value) in vars {
                    let j: Vec<isize> = key.chars().map(|i| ord(i).expect("")).collect();
                    r.extend(j);
                    r.push(ord('=')?);
                    let j: Vec<isize> = value.chars().map(|i| ord(i).expect("")).collect();
                    r.extend(j);
                    r.push(0);
                }
                r.push(0);
                r.reverse();
                r
            }
            i => {
                let j = i as usize - 20;
                let l = self.stack.len();
                if l >= j {
                    vec![self.stack.stackstack[0][l - j]]
                } else {
                    vec![0]
                }
            }
        })
    }

    fn not_implemented(&mut self, funge: &Funge) {
        println!("operator {} at {} not implemented", self.op(funge), join(&self.position, ", "));
        self.reverse()
    }

    fn step(mut self, mut funge: Funge, k: bool) -> Result<(Funge, Option<Vec<Self>>), Box<dyn Error>> {
        let mut new_ips = Vec::new();
        if self.string {
            match self.op(&funge) {
                34 => { self.string = false }  // '
                s => { self.stack.push(s) }
            }
        } else if self.fingerprint_ops.contains_key(&self.op(&funge)) {
            // self.fingerprint_ops[self.op(funge)]?
        } else if (0 <= self.op(&funge)) & (self.op(&funge) < 255) {
            match self.op(&funge) {
                43 => { // +
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(a + b);
                }
                45 => { // -
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(a - b);
                }
                42 => { // *
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(a * b);
                }
                47 => { // /
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(a / b);
                }
                37 => { // %
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(a % b);
                }
                33 => { // !
                    let a = self.stack.pop();
                    self.stack.push(!a as isize);
                }
                96 => { // `
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push((a > b) as isize);
                }
                62 => self.delta = vec![1, 0], // >
                60 => self.delta = vec![-1, 0], // <
                94 => self.delta = vec![0, -1], // ^
                118 => self.delta = vec![0, 1], // v
                63 => { // ?
                    let mut rng = rand::thread_rng();
                    self.delta = match rng.gen_range(0..4) {
                        0 => { vec![-1, 0] }
                        1 => { vec![1, 0] }
                        2 => { vec![0, -1] }
                        _ => { vec![0, 1] }
                    };
                }
                95 => { // _
                    if self.stack.pop() == 0 {
                        self.delta = vec![1, 0]
                    } else {
                        self.delta = vec![-1, 0]
                    }
                }
                124 => { // |
                    if self.stack.pop() == 0 {
                        self.delta = vec![0, 1];
                    } else {
                        self.delta = vec![0, -1];
                    }
                }
                34 => self.string = true, // "
                58 => { // :
                    let v = self.stack.pop();
                    self.stack.push(v);
                    self.stack.push(v);
                }
                92 => { // \
                    let a = self.stack.pop();
                    let b = self.stack.pop();
                    self.stack.push(a);
                    self.stack.push(b);
                }
                36 => { self.stack.pop(); } // $
                46 => funge.output.print(format!("{} ", self.stack.pop())), // .
                44 => funge.output.print(format!("{}", chr(self.stack.pop())?)), // ,
                35 => self.movep(&funge), // #
                112 => { // p
                    let y = self.stack.pop();
                    let x = self.stack.pop();
                    let v = self.stack.pop();
                    funge.insert(v, x + self.offset[0], y + self.offset[1]);
                }
                103 => { // g
                    let y = self.stack.pop();
                    let x = self.stack.pop();
                    self.stack.push(funge.code[&vec![x + self.offset[0], y + self.offset[1]]]);
                }
                38 => { // &
                    let s = funge.inputs.get()?;
                    let i: Vec<char> = s.chars()
                        .skip_while(|i| !i.is_digit(10))
                        .take_while(|i| i.is_digit(10)).collect();
                    self.stack.push(join(&i, "").parse()?);
                }
                126 => { // ~
                    let s = funge.inputs.get()?;
                    self.stack.push(ord(s.chars().nth(0).ok_or("No valid input.")?)?);
                }
                64 => { return Ok((funge, Some(Vec::new()))); } // @
                32 => { // space
                    self.advance(&funge);
                    return self.step(funge, false);
                }
                // 98 from here
                91 => self.turn_left(), // [
                93 => self.turn_right(), // ]
                39 => { // '
                    self.movep(&funge);
                    self.stack.push(self.op(&funge));
                }
                123 => { // {
                    let n = self.stack.pop();
                    let cells = if n > 0 {
                        let mut cells = Vec::new();
                        for _ in 0..n {
                            cells.push(self.stack.pop());
                        }
                        cells.reverse();
                        cells
                    } else {
                        vec![0; -n as usize]
                    };
                    for coordinate in &self.offset {
                        self.stack.push(*coordinate);
                    }
                    self.stack.push_stack(Stack::new());
                    for cell in cells {
                        self.stack.push(cell);
                    }
                    self.offset = self.next_pos(&funge);
                }
                125 => { // }
                    let n = self.stack.pop();
                    let cells = if n > 0 {
                        let mut cells = Vec::new();
                        for _ in 0..n {
                            cells.push(self.stack.pop());
                        }
                        cells.reverse();
                        cells
                    } else {
                        vec![0; -n as usize]
                    };
                    self.stack.pop_stack();
                    let y = self.stack.pop();
                    let x = self.stack.pop();
                    self.offset = vec![x, y];
                    for cell in cells {
                        self.stack.push(cell);
                    }
                }
                61 => { // =
                    self.reverse();
                    // self.stack.push(syscall(self.read_string()));
                }
                40 => { // ( no fingerprints are implemented
                    // self.read_fingerprint();
                    // self.fingerprint_ops[] = self.reverse;
                    self.reverse();
                }
                41 => { // )
                    // self.read_fingerprint()
                    // self.fingerprint_ops.pop()
                    self.reverse();
                }
                105 => { // i
                    let file = self.read_string()?;
                    let flags = self.stack.pop();
                    let y0 = self.stack.pop();
                    let x0 = self.stack.pop();
                    let text = fs::read_to_string(file)?;
                    let (width, height) = if flags % 2 != 0 {
                        let code: Vec<char> = text.chars().collect();
                        funge.insert_code(vec![join(&code, "")], x0, y0)?;
                        (text.len(), 1)
                    } else {
                        let text: Vec<&str> = text.lines().collect();
                        let height = text.len();
                        let width = text.iter().map(|i| i.len()).min().ok_or("Cannot calculate width.")?;
                        let mut code: Vec<String> = Vec::new();
                        for line in text {
                            let a = format!("{}{}", line, join(&vec![" "; width - line.len()], ""));
                            code.push(a);
                        }
                        funge.insert_code(code, x0, y0)?;
                        (width, height)
                    };
                    self.stack.push(x0);
                    self.stack.push(y0);
                    self.stack.push(width as isize);
                    self.stack.push(height as isize);
                }
                106 => { // j
                    for _ in 0..self.stack.pop() {
                        self.movep(&funge);
                    }
                }
                107 => { // k
                    self.advance(&funge);
                    let n = self.stack.pop();
                    let mut ips = vec![self];
                    for _ in 0..n {
                        let mut new_ips = Vec::new();
                        for ip in ips {
                            funge = match ip.step(funge, true)? {
                                (f, None) => { return Ok((f, None)) }
                                (f, Some(ips)) => {
                                    new_ips.extend(ips);
                                    f
                                }
                            }
                        }
                        ips = new_ips;
                    }
                    return Ok((funge, Some(ips)))
                }
                110 => self.stack.clear(), // n
                111 => { // o
                    let file = self.read_string()?;
                    let flags = self.stack.pop();
                    let x0 = self.stack.pop();
                    let y0 = self.stack.pop();
                    let width = self.stack.pop();
                    let height = self.stack.pop();
                    let mut text = Vec::new();
                    if flags % 2 != 0 {
                        for x in x0..x0 + width {
                            let mut line = String::new();
                            for y in y0..y0 + height {
                                line.push(chr(funge.code[&vec![x, y]])?);
                            }
                            line.rsplit(' ');
                            text.push(line);
                        }
                    } else {
                        for x in x0..x0 + width {
                            let mut line = String::new();
                            for y in y0..y0 + height {
                                line.push(chr(funge.code[&vec![x, y]])?);
                            }
                            text.push(line);
                        }
                    }
                    let text = join(&text, "\n");
                    fs::write(file, text)?;
                }
                113 => { return Ok((funge, None)) } // q
                114 => self.reverse(), // r
                115 => { // s
                    self.movep(&funge);
                    funge.insert(self.stack.pop(), self.position[0], self.position[1]);
                }
                116 => { // t
                    let mut new = self.copy(&funge);
                    new.reverse();
                    new_ips.push(new);
                }
                117 => { // u
                    if self.stack.is_empty() {
                        self.reverse();
                    } else {
                        let n = self.stack.pop();
                        let l = self.stack.len_stack();
                        if n > 0 {
                            for _ in 0..n {
                                let a = self.stack[l - 2].pop();
                                self.stack.push(a);
                            }
                        } else if n < 0 {
                            for _ in 0..-n {
                                let a = self.stack.pop();
                                self.stack[l - 2].push(a);
                            }
                        }
                    }
                }
                119 => { // w
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    if a < b {
                        self.turn_left();
                    } else if a > b {
                        self.turn_right();
                    }
                }
                120 => { // x
                    let dy = self.stack.pop();
                    let dx = self.stack.pop();
                    self.delta = vec![dx, dy];
                }
                121 => { // y
                    let n = self.stack.pop();
                    if n <= 0 {
                        for j in 1..21 {
                            for i in self.get_info(&funge,j) {
                                for cell in i {
                                    self.stack.push(cell);
                                }
                            }
                        }
                    } else {
                        for i in self.get_info(&funge, n) {
                            for cell in i {
                                self.stack.push(cell);
                            }
                        }
                    }
                }
                122 => { } // z
                d => {
                    if (48 <= d) & (d <= 57) { // 0123456789
                        self.stack.push(d - 48);
                    } else if (97 <= d) & (d <= 102) {
                        self.stack.push(d - 87);
                    } else {
                        self.not_implemented(&funge);
                    }
                }
            }
        } else {
            self.not_implemented(&funge);
        }
        if !k {
            self.advance(&funge);
        }
        let mut ips = vec![self];
        ips.extend(new_ips);
        Ok((funge, Some(ips)))
    }
}

#[derive(Clone)]
pub struct Funge {
    extent: Vec<isize>,
    code: HashMap<Vec<isize>, isize>,
    steps: isize,
    ips: Vec<IP>,
    inputs: Input,
    output: Output,
    pub terminated: bool
}

impl Funge {
    pub fn new<T: ToString>(code: T) -> Result<Self, Box<dyn Error>> {
        let mut new = Self {
            extent: vec![0, 0, 0, 0],
            code: HashMap::new(),
            steps: 0,
            ips: Vec::new(),
            inputs: Input { source: InputEnum::StdIn },
            output: Output { sink: OutputEnum::StdOut },
            terminated: false
        };
        let mut code: Vec<String> = code.to_string().lines().map(|i| String::from(i)).collect();
        if code[0].starts_with(r"#!/usr/bin/env befunge") | code[0].starts_with(r"#!/usr/bin/env -S befunge") {
            code.remove(0);
        }
        new.insert_code(code, 0, 0)?;
        new.ips.push(IP::new(&new)?);
        Ok(new)
    }

    pub fn from_file(file: &String) -> Result<Self, Box<dyn Error>> {
        Ok(Self::new(fs::read_to_string(file)?)?)
    }

    pub fn with_inputs(mut self, inputs: Vec<String>) -> Result<Self, Box<dyn Error>> {
        self.inputs = Input { source: InputEnum::Vector(inputs) };
        Ok(self)
    }

    pub fn with_output(mut self) -> Result<Self, Box<dyn Error>> {
        self.output = Output { sink: OutputEnum::Vector(Vec::new()) };
        Ok(self)
    }

    fn insert(&mut self, i: isize, x: isize, y: isize) {
        self.code.insert(vec![x, y], i);
    }

    fn insert_code(&mut self, code: Vec<String>, x0: isize, y0: isize) -> Result<(), Box<dyn Error>> {
        for (y, line) in code.iter().enumerate() {
            for (x, char) in line.chars().enumerate() {
                let x1: isize = x.try_into()?;
                let y1: isize = y.try_into()?;
                let position = vec![x0 + x1, y0 + y1];
                if position[0] < self.extent[0] {
                    self.extent[0] = position[0];
                } else if position[0] >= self.extent[1] {
                    self.extent[1] = position[0] + 1;
                }
                if position[1] < self.extent[2] {
                    self.extent[2] = position[1];
                } else if position[1] >= self.extent[3] {
                    self.extent[3] = position[1] + 1;
                }
                self.code.insert(position, ord(char)?);
            }
        }
        Ok(())
    }

    pub fn run(mut self) -> Result<Self, Box<dyn Error>>{
        while !self.terminated {
            self = self.step()?;
        }
        Ok(self)
    }

    pub fn step(mut self) -> Result<Self, Box<dyn Error>> {
        if !self.terminated {
            self.ips.reverse();
            let mut new_ips = Vec::new();
            for _ in 0..self.ips.len() {
                let ip = self.ips.pop().expect("");
                self = match ip.step(self, false)? {
                    (f, Some(ips)) => {
                        new_ips.extend(ips);
                        f
                    }
                    (mut f, None) => {
                        f.terminated = true;
                        return Ok(f)
                    }
                }
            }
            self.ips.extend(new_ips);
            self.steps += 1;
            if self.ips.len() == 0 {
                self.terminated = true;
            }
        }
        Ok(self)
    }

    pub fn ips_pos(&self) -> Vec<Vec<isize>> {
        let mut pos = Vec::new();
        for ip in self.ips.iter() {
            pos.push(ip.position.to_owned());
        }
        pos
    }

    fn to_string(&self, show_ips: bool) -> String {
        let mut lines = Vec::new();
        for (key, value) in (&self.code).into_iter() {
            let x= key[0] as usize;
            let y= key[1] as usize;
            while lines.len() <= y {
                lines.push(Vec::new());
            }
            while lines[y].len() <= x {
                lines[y].push(String::from(" "));
            }
            if ((32 <= *value) & (*value <= 126)) | ((161 <= *value) & (*value <= 255)) {
                lines[y][x] = chr(*value).unwrap().to_string();
            } else {
                lines[y][x] = chr(164).unwrap().to_string();
            }
        }

        if show_ips {
            for ip in &self.ips {
                let x = ip.position[0] as usize;
                let y = ip.position[1] as usize;
                lines[y][x] = format!("\x1b[37m\x1b[40m{}\u{001b}[0m", lines[y][x]);
            }
        }

        let mut string = String::from("grid:\n");
        string.push_str(&join(&lines.iter().map(|i| join(&i, "")).collect(), "\n"));
        string.push_str("\n\nstacks:\n");
        for ip in &self.ips {
            string.push_str(&ip.stack.to_string());
        }

        match &self.output.sink {
            OutputEnum::StdOut => { },
            OutputEnum::Vector(v) => {
                string.push_str("\n\nOutput:\n");
                string.push_str(&*join(&v, ""));
            }
        };

        string.push_str("\n\nsteps:\n");
        string.push_str(&self.steps.to_string());
        string
    }
}

impl Display for Funge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string(false))
    }
}

impl Debug for Funge {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_string(true))
    }
}