use std::collections::HashMap;
use std::{env, fs, fmt, fmt::{Debug, Display, Formatter}};
use std::ops::{Add, Deref, DerefMut, Index, IndexMut, Sub};
use std::{error::Error, hash::Hash, path::Path, str::FromStr, io::stdin};
use chrono::{offset::Local, {Datelike, Timelike}};
use rand::Rng;
use num::{Integer, NumCast};


const VERSION: &str = env!("CARGO_PKG_VERSION");


pub trait Int: Integer + NumCast + FromStr + Hash + Clone + Copy + Sync + Send + Display + 'static {}
impl<I: Integer + NumCast + FromStr + Hash + Clone + Copy + Sync + Send + Display + 'static> Int for I {}


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

fn cast_int<I: NumCast, J: NumCast>(j: J) -> Result<I, Box<dyn Error>> {
    Ok(I::from(j).ok_or("Could not convert from primitive")?)
}

fn cast_vec_int<I: NumCast, J: NumCast>(j: Vec<J>) -> Result<Vec<I>, Box<dyn Error>> {
    let mut i = Vec::<I>::new();
    for n in j {
        i.push(cast_int(n)?);
    }
    Ok(i)
}

fn ord<I: NumCast>(c: char) -> Result<I, Box<dyn Error>>
{
    Ok(cast_int::<_, u32>(c.try_into()?)?)
}

fn chr<I: NumCast>(i: I) -> Result<char, Box<dyn Error>> {
    Ok(cast_int::<u32, _>(i)?.try_into()?)
}

fn add<I: Add + Copy>(a: &Vec<I>, b: &Vec<I>) -> Vec<I> where
    Vec<I>: FromIterator<<I as Add>::Output> {
    a.iter().zip(b.iter()).map(|(&a, &b)| a + b).collect()
}

fn sub<I: Sub + Copy>(a: &Vec<I>, b: &Vec<I>) -> Vec<I> where
    Vec<I>: FromIterator<<I as Sub>::Output> {
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
            OutputEnum::StdOut => print!("{}", string),
            OutputEnum::Vector(ref mut v) => v.push(string)
        }
    }
}


#[derive(Clone)]
struct Stack<I: Int> {
    stack: Vec<I>
}


impl<I: Int> Stack<I> {
    fn new() -> Self {
        Self { stack: Vec::new() }
    }

    fn pop(&mut self) -> I {
        match self.stack.pop() {
            Some(value) => { value }
            None => { I::zero() }
        }
    }

    fn push(&mut self, cell: I) {
        self.stack.push(cell)
    }

    fn len(&self) -> usize { self.stack.len() }
}

impl<I: Int> Display for Stack<I> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "[{}]", join(&self.stack, ", "))
    }
}

impl<I: Int> Index<usize> for Stack<I> {
    type Output = I;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stack[index]
    }
}

#[derive(Clone)]
struct StackStack<I: Int> {
    stackstack: Vec<Stack<I>>
}

impl<I: Int> StackStack<I> {
    fn new() -> Self {
        Self { stackstack: vec![Stack::new()] }
    }

    fn check_stack(&mut self) {
        if self.is_empty() {
            self.push_stack(Stack::new());
        }
    }

    fn pop(&mut self) -> I {
        self.check_stack();
        let x = self.len_stack();
        self.stackstack[x - 1].pop()
    }

    fn push(&mut self, cell: I) {
        self.check_stack();
        let x = self.len_stack();
        self.stackstack[x - 1].push(cell);
    }

    fn pop_stack(&mut self) -> Stack<I> {
        match self.stackstack.pop() {
            Some(stack) => { stack }
            None => { Stack::new() }
        }
    }

    fn push_stack(&mut self, stack: Stack<I>) {
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

impl<I: Int> Index<usize> for StackStack<I> {
    type Output = Stack<I>;

    fn index(&self, index: usize) -> &Self::Output {
        &self.stackstack[index]
    }
}

impl<I: Int> IndexMut<usize> for StackStack<I> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.stackstack[index]
    }
}

impl<I: Int> Display for StackStack<I> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(f, "{}", join(&self.stackstack, "\n"))
    }
}


#[derive(Clone)]
struct IP<I: Int> {
    id: usize,
    position: Vec<isize>,
    delta: Vec<isize>,
    offset: Vec<isize>,
    string: bool,
    stack: StackStack<I>,
    fingerprint_ops: HashMap<I, ()>,
}


impl<I: Int> IP<I> {
    fn new(funge: &Funge<I>) -> Result<Self, Box<dyn Error>> {
        let mut new = IP {
            id: funge.ips.len(),
            position: vec![0, 0],
            delta: vec![1, 0],
            offset: vec![0, 0],
            string: false,
            stack: StackStack::new(),
            fingerprint_ops: HashMap::new()
        };
        if let Ok(32 | 59) = cast_int(new.op(funge)) {
            new.advance(funge)?;
        };
        Ok(new)
    }

    fn copy(&self, funge: &Funge<I>) -> Self {
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

    fn op(&self, funge: &Funge<I>) -> I {
        funge.code[&self.position]
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

    fn advance(&mut self, funge: &Funge<I>) -> Result<(), Box<dyn Error>> {
        let space: I = cast_int(32)?;
        let semicolon: I = cast_int(59)?;
        Ok(if self.string {
            if self.op(funge) == space {
                while self.op(funge) == space {
                    self.movep(funge)
                }
            } else {
                self.movep(funge)
            }
        } else {
            loop {
                if self.op(funge) != semicolon {
                    self.movep(funge)
                }
                if self.op(funge) == semicolon {
                    self.movep(funge);
                    while self.op(funge) != semicolon {
                        self.movep(funge)
                    }
                    self.movep(funge)
                }
                while self.op(funge) == space {
                    self.movep(funge)
                }
                if self.op(funge) != semicolon {
                    break;
                }
            }
        })
    }

    fn movep(&mut self, funge: &Funge<I>) {
        self.position = self.next_pos(funge);
    }

    fn skip(mut self, funge: Funge<I>) -> Result<(Funge<I>, Option<Vec<Self>>), Box<dyn Error>> {
        self.movep(&funge);
        if let Ok(32 | 59) = cast_int(self.op(&funge)) {
            self.advance(&funge)?;
        };
        return Ok((funge, Some(vec![self])))
    }

    fn check_pos(&self, pos: &Vec<isize>, funge: &Funge<I>) -> bool {
        (funge.extent[0] <= pos[0]) & (pos[0] < funge.extent[1]) &
            (funge.extent[2] <= pos[1]) & (pos[1] < funge.extent[3])
    }

    fn next_pos(&self, funge: &Funge<I>) -> Vec<isize> {
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
            if f == I::zero() {
                return Ok(string)
            } else {
                string.push_str(&chr(f)?.to_string())
            }
        }
    }

    fn get_info(&self, funge: &Funge<I>, n: I) -> Result<Vec<I>, Box<dyn Error>> {
        let time = Local::now();
        match n.to_usize() {
            Some(n @ 1..=20) => {
                Ok(match n {
                    1 => vec![cast_int(15)?],
                    2 => vec![cast_int(8 * std::mem::size_of::<I>())?],
                    3 => {
                        let mut f = 0;
                        for (i, c) in "wprustyfunge".chars().enumerate() {
                            f += (256 as isize).pow(i as u32) * ord::<isize>(c)?;
                        }
                        vec![cast_int(f)?]
                    }
                    4 => vec![cast_int(VERSION.replace(".", "").parse::<isize>()?)?],
                    5 => vec![I::one()],
                    6 => vec![ord(std::path::MAIN_SEPARATOR)?],
                    7 => vec![cast_int(2)?],
                    8 => vec![cast_int(*&self.id)?],
                    9 => vec![I::zero()],
                    10 => cast_vec_int(self.position.to_owned())?,
                    11 => cast_vec_int(self.delta.to_owned())?,
                    12 => cast_vec_int(self.offset.to_owned())?,
                    13 => cast_vec_int(funge.extent.chunks(2).map(|i| i[0]).collect())?,
                    14 => cast_vec_int(funge.extent.chunks(2).map(|i| i[1]).collect())?,
                    15 => vec![cast_int((time.year() - 1900) * 256 * 256 + (time.month() as i32) * 256 + (time.day() as i32))?],
                    16 => vec![cast_int(time.hour() * 256 * 256 + time.minute() * 256 + time.second())?],
                    17 => vec![cast_int(self.stack.len_stack())?],
                    18 => {
                        let mut l = Vec::new();
                        for stack in &self.stack.stackstack {
                            l.push(cast_int(stack.len())?);
                        }
                        l.reverse();
                        l
                    }
                    19 => {
                        let mut r = Vec::new();
                        let args: Vec<String> = env::args().collect();
                        if args.len() > 1 {
                            for i in 1..args.len() {
                                let j: Vec<I> = args[i].chars().map(|i| ord(i).expect("")).collect();
                                r.extend(j);
                                r.push(I::zero());
                            }
                        }
                        r.push(I::zero());
                        let file = &args[0];
                        let path = Path::new(&file);
                        let j: Vec<I> = path.file_name().ok_or("No file name.")?
                            .to_str().ok_or("Cannot convert String.")?
                            .chars().map(|i| ord(i).expect("")).collect();
                        r.extend(j);
                        r.push(I::zero());
                        r.push(I::zero());
                        r.reverse();
                        r
                    }
                    20 => {
                        let mut r = Vec::new();
                        let vars = env::vars();
                        for (key, value) in vars {
                            let j: Vec<I> = key.chars().map(|i| ord(i).expect("")).collect();
                            r.extend(j);
                            r.push(ord('=')?);
                            let j: Vec<I> = value.chars().map(|i| ord(i).expect("")).collect();
                            r.extend(j);
                            r.push(I::zero());
                        }
                        r.push(I::zero());
                        r.reverse();
                        r
                    }
                    i => {
                        let j = i as usize - 20;
                        let l = self.stack.len();
                        if l >= j {
                            vec![self.stack.stackstack[0][l - j]]
                        } else {
                            vec![I::zero()]
                        }
                    }
                })
            }
            _ => {
                // TODO: return Error
                println!("{}", "Stack size overflow");
                Ok(Vec::new())
            }
        }
    }

    fn not_implemented(&mut self, funge: &Funge<I>) {
        // TODO: reverse or quit option
        println!("operator {} at {} not implemented", self.op(funge), join(&self.position, ", "));
        self.reverse()
    }

    fn step(mut self, mut funge: Funge<I>, k: bool) -> Result<(Funge<I>, Option<Vec<Self>>), Box<dyn Error>> {
        let mut new_ips = Vec::new();
        let op = self.op(&funge);
        let op8 = op.to_u8();
        if self.string {
            match op8 {
                Some(34) => { self.string = false }  // "
                _ => { self.stack.push(op) }
            }
        } else if self.fingerprint_ops.contains_key(&op) {
            // self.fingerprint_ops[self.op(funge)]?
        } else if let Some(0..=255) = op8 {
            match op8.expect("Could not convert.") {
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
                    if b == I::zero() {
                        self.stack.push(I::zero());
                    } else {
                        self.stack.push(a / b);
                    }
                }
                37 => { // %
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    self.stack.push(a % b);
                }
                33 => { // !
                    let a = self.stack.pop();
                    if a == I::zero() {
                        self.stack.push(I::one());
                    } else {
                        self.stack.push(I::zero());
                    }
                }
                96 => { // `
                    let b = self.stack.pop();
                    let a = self.stack.pop();
                    if a > b {
                        self.stack.push(I::one());
                    } else {
                        self.stack.push(I::zero());
                    }
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
                    if self.stack.pop() == I::zero() {
                        self.delta = vec![1, 0]
                    } else {
                        self.delta = vec![-1, 0]
                    }
                }
                124 => { // |
                    if self.stack.pop() == I::zero() {
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
                35 => { // #
                    self.movep(&funge);
                    return self.skip(funge)
                }
                112 => { // p
                    let y: isize = cast_int(self.stack.pop())?;
                    let x: isize = cast_int(self.stack.pop())?;
                    let v = self.stack.pop();
                    funge.insert(v, x + self.offset[0], y + self.offset[1]);
                }
                103 => { // g
                    let y: isize = cast_int(self.stack.pop())?;
                    let x: isize = cast_int(self.stack.pop())?;
                    self.stack.push(*&funge.code[&vec![x + self.offset[0], y + self.offset[1]]]);
                }
                38 => { // &
                    let s = funge.inputs.get()?;
                    let i: Vec<char> = s.chars()
                        .skip_while(|i| !i.is_digit(10))
                        .take_while(|i| i.is_digit(10)).collect();
                    match join(&i, "").parse() {
                        Ok(n) => self.stack.push(n),
                        _ => println!("Cannot convert input to number.")  // TODO: Error
                    }
                }
                126 => { // ~
                    let s = funge.inputs.get()?;
                    self.stack.push(ord(s.chars().nth(0).ok_or("No valid input.")?)?);
                }
                64 => { return Ok((funge, Some(Vec::new()))); } // @
                32 => { // space
                    self.advance(&funge)?;
                    return Ok(self.step(funge,false)?);
                }
                // 98 from here
                91 => self.turn_left(), // [
                93 => self.turn_right(), // ]
                39 => { // '
                    self.movep(&funge);
                    self.stack.push(self.op(&funge));
                    return self.skip(funge)
                }
                123 => { // {
                    let n = self.stack.pop();
                    let cells = if n > I::zero() {
                        let mut cells = Vec::new();
                        for _ in 0..cast_int(n)? {
                            cells.push(self.stack.pop());
                        }
                        cells.reverse();
                        cells
                    } else {
                        vec![I::zero(); -cast_int::<isize, _>(n)? as usize]
                    };
                    for coordinate in &self.offset {
                        self.stack.push(cast_int(*coordinate)?);
                    }
                    self.stack.push_stack(Stack::new());
                    for cell in cells {
                        self.stack.push(cell);
                    }
                    self.offset = self.next_pos(&funge);
                }
                125 => { // }
                    let n = self.stack.pop();
                    let cells = if n > I::zero() {
                        let mut cells = Vec::new();
                        for _ in 0..cast_int(n)? {
                            cells.push(self.stack.pop());
                        }
                        cells.reverse();
                        cells
                    } else {
                        vec![I::zero(); -cast_int::<isize, _>(n)? as usize]
                    };
                    self.stack.pop_stack();
                    let y = cast_int(self.stack.pop())?;
                    let x = cast_int(self.stack.pop())?;
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
                    let y0 = cast_int(self.stack.pop())?;
                    let x0 = cast_int(self.stack.pop())?;
                    let text = fs::read_to_string(file)?;
                    let (width, height) = if flags.is_odd() {
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
                    self.stack.push(cast_int(x0)?);
                    self.stack.push(cast_int(y0)?);
                    self.stack.push(cast_int(width)?);
                    self.stack.push(cast_int(height)?);
                }
                106 => { // j
                    for _ in 0..cast_int(self.stack.pop())? {
                        self.movep(&funge);
                    }
                    return self.skip(funge)
                }
                107 => { // k
                    self.advance(&funge)?;
                    let n = cast_int(self.stack.pop())?;
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
                    let x0 = cast_int(self.stack.pop())?;
                    let y0 = cast_int(self.stack.pop())?;
                    let width: isize = cast_int(self.stack.pop())?;
                    let height: isize = cast_int(self.stack.pop())?;
                    let mut text = Vec::new();
                    if flags.is_odd() {
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
                        let n = cast_int(self.stack.pop())?;
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
                    let dy = cast_int(self.stack.pop())?;
                    let dx = cast_int(self.stack.pop())?;
                    self.delta = vec![dx, dy];
                }
                121 => { // y
                    let n = self.stack.pop();
                    if n <= I::zero() {
                        for j in 1..21 {
                            for i in self.get_info(&funge,cast_int(j)?) {
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
                48..=57 => self.stack.push(self.op(&funge) - cast_int(48)?), // 0123456789
                97..=102 => self.stack.push(self.op(&funge) - cast_int(87)?), // abcdef
                _ => self.not_implemented(&funge)
            }
        } else {
            self.not_implemented(&funge);
        }
        if !k {
            self.advance(&funge)?;
        }
        let mut ips = vec![self];
        ips.extend(new_ips);
        Ok((funge, Some(ips)))
    }
}


#[derive(Clone)]
struct DefaultHashMap<K: Eq + Hash, V: Clone> {
    hashmap: HashMap<K, V>,
    default: V
}

impl<K: Eq + Hash, V: Clone> DefaultHashMap<K, V> {
    fn new(default: V) -> Self {
        Self { hashmap: HashMap::new(), default }
    }
}

impl<K: Eq + Hash, V: Clone> Deref for DefaultHashMap<K, V> {
    type Target = HashMap<K, V>;

    fn deref(&self) -> &Self::Target {
        &self.hashmap
    }
}

impl<K: Eq + Hash, V: Clone> DerefMut for DefaultHashMap<K, V> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.hashmap
    }
}

impl<K: Eq + Hash, V: Clone> Index<&K> for DefaultHashMap<K, V> {
    type Output = V;

    fn index(&self, index: &K) -> &Self::Output {
        self.hashmap.get(&index).unwrap_or(&self.default)
    }
}


#[derive(Clone)]
pub struct Funge<I: Int> {
    extent: Vec<isize>,
    code: DefaultHashMap<Vec<isize>, I>,
    steps: isize,
    ips: Vec<IP<I>>,
    inputs: Input,
    output: Output,
    pub terminated: bool
}

impl<I: Int> Funge<I> {
    pub fn new<T: ToString>(code: T) -> Result<Self, Box<dyn Error>> {
        let mut new = Self {
            extent: vec![0; 4],
            code: DefaultHashMap::new(cast_int(32)?),
            steps: 0,
            ips: Vec::new(),
            inputs: Input { source: InputEnum::StdIn },
            output: Output { sink: OutputEnum::StdOut },
            terminated: false
        };
        let mut code: Vec<String> = code.to_string().lines().map(|i| String::from(i)).collect();
        let exe = env::current_exe()?.file_name().ok_or("No exe name")?.to_str().unwrap().to_string();
        if code[0].starts_with(&*format!(r"#!/usr/bin/env {}", exe)) | code[0].starts_with(&*format!(r"#!/usr/bin/env -S {}", exe)) {
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

    fn insert(&mut self, i: I, x: isize, y: isize) {
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

    fn get_string(&self, show_ips: bool) -> String {
        let mut lines = Vec::new();
        for (key, value) in (&*self.code).into_iter() {
            let x= key[0] as usize;
            let y= key[1] as usize;
            while lines.len() <= y {
                lines.push(Vec::new());
            }
            while lines[y].len() <= x {
                lines[y].push(String::from(" "));
            }
            if let Some(32..=126) | Some(161..=255) = value.to_u8() {
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
        string.push_str(&join(&(&self.ips).iter().map(|ip| ip.stack.to_string()).collect(), "\n"));

        match &self.output.sink {
            OutputEnum::StdOut => { },
            OutputEnum::Vector(v) => {
                string.push_str("\n\nOutput:\n");
                string.push_str(&join(&v, ""));
            }
        };

        string.push_str("\n\nsteps:\n");
        string.push_str(&self.steps.to_string());
        string
    }
}

impl<I: Int> Display for Funge<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_string(false))
    }
}

impl<I: Int> Debug for Funge<I> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.get_string(true))
    }
}