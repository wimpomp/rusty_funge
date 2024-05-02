use std::collections::HashMap;
use std::{env, fs, fmt, fmt::{Debug, Display, Formatter}, io};
use std::ops::{Add, Index, IndexMut, Sub};
use std::{hash::Hash, path::Path, str::FromStr, io::stdin};
use std::cmp::{max, min};
use std::process::Command;
use std::io::Write;
use anyhow::{Error, Result};
use chrono::{offset::Local, {Datelike, Timelike}};
use rand::Rng;
use num::{Integer, NumCast};
use strum_macros::EnumString;
use regex::Regex;


const VERSION: &str = env!("CARGO_PKG_VERSION");


pub trait Int: Integer + NumCast + FromStr + Hash + Clone + Copy + Sync + Send + Display + 'static {}
impl<I: Integer + NumCast + FromStr + Hash + Clone + Copy + Sync + Send + Display + 'static> Int for I {}


#[derive(Debug, thiserror::Error)]
enum FungeError {
    #[error("Could not convert from primitive.")]
    Casting,
    #[error("No file name.")]
    FileName,
    #[error("Cannot convert String.")]
    String,
    #[error("Invalid input.")]
    Input,
    #[error("Unrecognized version: {0}")]
    Version(String),
    #[error("Funge exited with return code {0}.")]
    Quit(i32)
}


fn split_string(string: String) -> Result<Vec<String>> {
    let mut string = string.as_str();
    let mut _mat = "";
    let mut res = Vec::new();
    let r = Regex::new(r"[^\\](\s)")?;
    loop {
        if let Some(m) = r.find(&string) {
            (_mat, string) = string.split_at(m.end());
            res.push(_mat.trim().replace(r"\ ", " ").to_string());
        } else {
            res.push(string.trim().replace(r"\ ", " ").to_string());
            break
        }
    }
    Ok(res)
}



pub fn join<T: ToString>(v: &Vec<T>, s: &str) -> String {
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

pub fn cast_int<I: NumCast, J: NumCast>(j: J) -> Result<I> {
    Ok(I::from(j).ok_or(Error::new(FungeError::Casting))?)
}

fn cast_vec_int<I: NumCast, J: NumCast>(j: Vec<J>) -> Result<Vec<I>> {
    let mut i = Vec::<I>::new();
    for n in j {
        i.push(cast_int(n)?);
    }
    Ok(i)
}

fn vec_to_string<I: NumCast>(vec: Vec<I>) -> String {
    join(&vec.into_iter().map(|i| {
        chr(match cast_int::<u8, I>(i) {
            Ok(n @ 32..=126) | Ok(n @ 161..=255) => n,
            _ => 164
        }).expect("These values should work.")
    }).collect::<Vec<char>>(), "")
}

pub fn ord<I: NumCast>(c: char) -> Result<I> {
    Ok(cast_int::<_, u32>(c.try_into()?)?)
}

pub fn chr<I: NumCast>(i: I) -> Result<char> {
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
pub struct IO {
    pub store: Vec<String>,
    input: fn(&mut Vec<String>) -> Result<String>,
    output: fn(&mut Vec<String>, String) -> Result<()>,
}

impl IO {
    pub fn new() -> Self {
        Self {
            store: Vec::new(),
            input: |store| {
                Ok(match store.pop() {
                    None => {
                        let mut s = String::new();
                        stdin().read_line(&mut s)?;
                        s
                    }
                    Some(s) => s
                })
            },
            output: |_, s| {
                print!("{}", s);
                io::stdout().flush().unwrap_or(());
                Ok(())
            }
        }
    }

    pub fn with_store(mut self, mut store: Vec<String>) -> Self {
        store.reverse();
        self.store = store;
        self
    }

    pub fn with_input(mut self, fun: fn(&mut Vec<String>) -> Result<String>) -> Self {
        self.input = fun;
        self
    }

    pub fn with_output(mut self, fun: fn(&mut Vec<String>, String) -> Result<()>) -> Self {
        self.output = fun;
        self
    }

    pub fn len(&self) -> usize {
        self.store.len()
    }

    fn pop(&mut self) -> Result<String> {
        (self.input)(&mut self.store)
    }

    fn push(&mut self, s: String) -> Result<()> {
        (self.output)(&mut self.store, s)
    }

    pub fn get(&self) -> String {
        join(&self.store, "")
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
            Some(value) => value,
            None => I::zero()
        }
    }

    fn push(&mut self, cell: I) {
        self.stack.push(cell)
    }

    fn extend(&mut self, cells: Vec<I>) {
        self.stack.extend(cells);
    }

    fn len(&self) -> usize {
        self.stack.len()
    }
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

    fn extend(&mut self, cells: Vec<I>) {
        self.check_stack();
        let x = self.len_stack();
        self.stackstack[x - 1].extend(cells);
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
pub struct IP<I: Int> {
    pub id: usize,
    position: Vec<isize>,
    delta: Vec<isize>,
    pub offset: Vec<isize>,
    string: bool,
    stack: StackStack<I>,
    fingerprint_ops: HashMap<I, ()>
}


impl<I: Int> IP<I> {
    fn new(funge: &Funge<I>) -> Result<Self> {
        let mut new = IP {
            id: 0,
            position: vec![0, 0],
            delta: vec![1, 0],
            offset: vec![0, 0],
            string: false,
            stack: StackStack::new(),
            fingerprint_ops: HashMap::new()
        };
        if let Ok(32 | 59) = cast_int(new.op(funge)) {
            new = new.advance(funge, false)?;
        };
        Ok(new)
    }

    fn split(&self, id: usize) -> Self {
        Self {
            id: id,
            position: self.position.to_owned(),
            delta: self.delta.to_owned(),
            offset: self.offset.to_owned(),
            string: self.string,
            stack: self.stack.to_owned(),
            fingerprint_ops: self.fingerprint_ops.to_owned()
        }
    }

    fn op(&self, funge: &Funge<I>) -> I {
        self.op_at(funge, &self.position)
    }

    fn op_at(&self, funge: &Funge<I>, pos: &Vec<isize>) -> I {
        funge.code[pos]
    }

    fn next_op(&self, funge: &Funge<I>) -> Result<I> {
        let next_pos = self.next_valid_pos(&funge, false)?;
        Ok(funge.code[&next_pos])
    }

    fn reflect(&mut self) {
        self.delta = self.delta.iter().map(|i| -i).collect();
    }

    fn turn_right(&mut self) {
        self.delta = vec![-self.delta[1], self.delta[0]];
    }

    fn turn_left(&mut self) {
        self.delta = vec![self.delta[1], -self.delta[0]];
    }

    fn advance(mut self, funge: &Funge<I>, skip: bool) -> Result<Self> {
        self.position = self.next_valid_pos(funge, skip)?;
        Ok(self)
    }

    fn movep(&mut self, funge: &Funge<I>) {
        self.position = self.next_pos(funge, self.position.to_owned());
    }

    fn check_pos(&self, pos: &Vec<isize>, funge: &Funge<I>) -> bool {
        (funge.extent.left <= pos[0]) & (pos[0] < funge.extent.right) &
            (funge.extent.top <= pos[1]) & (pos[1] < funge.extent.bottom)
    }

    fn next_valid_pos(&self, funge: &Funge<I>, skip: bool) -> Result<Vec<isize>> {
        let mut pos = self.position.to_owned();
        let space: I = cast_int(32)?;
        let semicolon: I = cast_int(59)?;
        if self.string {
            if self.op_at(funge, &pos) == space {
                while self.op_at(funge, &pos) == space {
                    pos = self.next_pos(funge, pos);
                }
            } else {
                pos = self.next_pos(funge, pos);
            }
        } else {
            if (self.op_at(funge, &pos) != semicolon) | skip {
                pos = self.next_pos(funge, pos);
            }
            loop {
                if self.op_at(funge, &pos) == semicolon {
                    pos = self.next_pos(funge, pos);
                    while self.op_at(funge, &pos) != semicolon {
                        pos = self.next_pos(funge, pos);
                    }
                    pos = self.next_pos(funge, pos);
                }
                while self.op_at(funge, &pos) == space {
                    pos = self.next_pos(funge, pos);
                }
                if self.op_at(funge, &pos) != semicolon {
                    break;
                }
            }
        }
        Ok(pos)
    }

    fn next_pos(&self, funge: &Funge<I>, mut pos: Vec<isize>) -> Vec<isize> {
        if self.check_pos(&pos, funge) {  // always do one step outside before wrapping
            add(&pos, &self.delta)
        } else {
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
    }

    fn read_string(&mut self) -> Result<String> {
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

    fn read_fingerprint(&mut self) -> Result<()> {
        for _ in 0..cast_int(self.stack.pop())? {
            self.stack.pop();
        }
        Ok(())
    }

    fn get_info(&mut self, funge: &Funge<I>) -> Result<usize> {
        let time = Local::now();
        let vars = env::vars();
        let mut l = Vec::new();
        let size = self.stack.len();
        for stack in &self.stack.stackstack {
            l.push(cast_int(stack.len())?);
        }
        let mut f = 0;
        for (i, c) in "wprusty".chars().enumerate() {
            f += (256 as isize).pow(i as u32) * ord::<isize>(c)?;
        }
        let mut flags = 16;  // unbuffered IO
        for (i, op) in [116, 105, 111, 61].iter().enumerate() {  // tio=
            if funge.rules.instruction_set.contains(op) {
                flags += 2isize.pow(i as u32);
            }
        }

        let mut r = Vec::new();
        for (key, value) in vars {
            let j: Vec<I> = key.chars().map(|i| ord(i).expect("")).collect();
            r.extend(j);
            r.push(ord('=')?);
            let j: Vec<I> = value.chars().map(|i| ord(i).expect("")).collect();
            r.extend(j);
            r.push(I::zero());
        }
        r.push(I::zero());
        r.push(I::zero());
        r.reverse();
        self.stack.extend(r);  // 20

        let mut r = Vec::new();
        let args: Vec<String> = env::args().collect();
        if args.len() > 1 {
            for i in 1..args.len() {
                let j: Vec<I> = args[i].chars().map(|i| ord(i).expect("")).collect();
                r.extend(j);
                r.push(I::zero());
            }
        }
        let file = &args[0];
        let path = Path::new(&file);
        let j: Vec<I> = path.file_name().ok_or(Error::new(FungeError::FileName))?
            .to_str().ok_or(Error::new(FungeError::String))?
            .chars().map(|i| ord(i).expect("")).collect();
        r.extend(j);
        r.push(I::zero());
        r.push(I::zero());
        r.push(I::zero());
        r.reverse();
        self.stack.extend(r);  // 19

        self.stack.extend(l);  // 18
        self.stack.push(cast_int(self.stack.len_stack())?);  // 17
        self.stack.push(cast_int(time.hour() * 256 * 256 + time.minute() * 256 + time.second())?);  // 16
        self.stack.push(cast_int((time.year() - 1900) * 256 * 256 + (time.month() as i32) * 256 + (time.day() as i32))?);  // 15
        self.stack.extend(cast_vec_int(vec![funge.extent.width() - 1, funge.extent.height() - 1])?);  // 14
        self.stack.extend(cast_vec_int(vec![funge.extent.left, funge.extent.top])?);  // 13
        self.stack.extend(cast_vec_int(self.offset.to_owned())?);  // 12
        self.stack.extend(cast_vec_int(self.delta.to_owned())?);  // 11
        self.stack.extend(cast_vec_int(self.position.to_owned())?);  // 10
        self.stack.push(I::zero());  // 9
        self.stack.push(cast_int(*&self.id)?);  // 8
        self.stack.push(cast_int(2)?);  // 7
        self.stack.push(cast_int(ord::<I>(std::path::MAIN_SEPARATOR)?)?);  // 6
        self.stack.push(I::one());  // 5
        self.stack.push(cast_int(VERSION.replace(".", "").parse::<isize>()?)?);  // 4
        self.stack.push(cast_int(f)?);  // 3
        self.stack.push(cast_int(std::mem::size_of::<I>())?);  // 2
        self.stack.push(cast_int(flags)?);  // 1

        Ok(self.stack.len() - size)
    }

    fn not_implemented(&mut self, funge: &Funge<I>) -> Result<()> {
        match funge.rules.on_error {
            OnError::Ignore => {
                Ok(())
            }
            OnError::Reflect => {
                self.reflect();
                Ok(())
            }
            OnError::Quit => Err(Error::new(FungeError::Quit(0)))
        }
    }

    fn step(self, funge: Funge<I>, n_ips: usize) -> Result<(Funge<I>, Vec<Self>)> {
        let op = self.op(&funge);
        let (funge, mut ips, skip) = self.exe(funge, op, n_ips)?;
        ips = ips.into_iter().map(|ip| ip.advance(&funge, skip)).collect::<Result<Vec<IP<I>>>>()?;
        Ok((funge, ips))
    }

    fn exe(mut self, mut funge: Funge<I>, op: I, n_ips: usize) -> Result<(Funge<I>, Vec<Self>, bool)> {
        let mut new_ips = Vec::new();
        if self.string {
            match op.to_u8() {
                Some(34) => { self.string = false }  // "
                _ => { self.stack.push(op) }
            }
        } else if self.fingerprint_ops.contains_key(&op) {
            // self.fingerprint_ops[self.op(funge)]?
        } else if let Some(n @ 0..=255) = op.to_u8() {
            if funge.rules.instruction_set.contains(&n) {
                match n {
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
                        if b == I::zero() {
                            self.stack.push(I::zero());
                        } else {
                            self.stack.push(a % b);
                        }
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
                    46 => funge.output.push(format!("{} ", self.stack.pop()))?, // .
                    44 => funge.output.push(format!("{}", chr(self.stack.pop())?))?, // ,
                    35 => { // #
                        self.movep(&funge);
                        return Ok((funge, vec![self], true))
                    }
                    112 => { // p
                        let y: isize = cast_int(self.stack.pop())?;
                        let x: isize = cast_int(self.stack.pop())?;
                        let v = self.stack.pop();
                        funge.insert(v, vec![x + self.offset[0], y + self.offset[1]]);
                    }
                    103 => { // g
                        let y: isize = cast_int(self.stack.pop())?;
                        let x: isize = cast_int(self.stack.pop())?;
                        self.stack.push(*&funge.code[&vec![x + self.offset[0], y + self.offset[1]]]);
                    }
                    38 => { // &
                        match funge.input.pop() {
                            Ok(s) => {  // TODO: take until input would cause cell overflow
                                let i: Vec<char> = s.chars()
                                    .skip_while(|i| !i.is_digit(10))
                                    .take_while(|i| i.is_digit(10)).collect();
                                match join(&i, "").parse() {
                                    Ok(n) => self.stack.push(n),
                                    _ => self.reflect()
                                }
                            }
                            Err(_) => self.reflect()
                        }
                    }
                    126 => { // ~
                        match funge.input.pop() {
                            Ok(s) => self.stack.push(ord(s.chars().nth(0).ok_or(Error::new(FungeError::Input))?)?),
                            Err(_) => self.reflect()
                        }
                    }
                    64 => return Ok((funge, Vec::new(), false)), // @
                    32 => { // space
                        self = self.advance(&funge, false)?;
                        let n_op = self.op(&funge);
                        return Ok(self.exe(funge, n_op, n_ips)?);
                    }
                    // 98 from here
                    91 => self.turn_left(), // [
                    93 => self.turn_right(), // ]
                    39 => { // '
                        self.movep(&funge);
                        self.stack.push(self.op(&funge));
                        return Ok((funge, vec![self], true))
                    }
                    123 => { // {
                        let n: isize = cast_int(self.stack.pop())?;
                        let cells = if n > 0 {
                            let mut cells = Vec::new();
                            for _ in 0..n {
                                cells.push(self.stack.pop());
                            }
                            cells.reverse();
                            cells
                        } else {
                            for _ in 0..-n {
                                self.stack.push(I::zero());
                            }
                            Vec::new()
                        };
                        for coordinate in &self.offset {
                            self.stack.push(cast_int(*coordinate)?);
                        }
                        self.stack.push_stack(Stack::new());
                        for cell in cells {
                            self.stack.push(cell);
                        }
                        self.offset = self.next_pos(&funge, self.position.to_owned());
                    }
                    125 => { // }
                        if self.stack.len_stack() <= 1 {
                            self.reflect()
                        } else {
                            let n: isize = cast_int(self.stack.pop())?;
                            let cells = if n > 0 {
                                let mut cells = Vec::new();
                                for _ in 0..n {
                                    cells.push(self.stack.pop());
                                }
                                cells.reverse();
                                cells
                            } else {
                                Vec::new()
                            };
                            self.stack.pop_stack();
                            let y = cast_int(self.stack.pop())?;
                            let x = cast_int(self.stack.pop())?;
                            self.offset = vec![x, y];
                            if n > 0 {
                                for cell in cells {
                                    self.stack.push(cell);
                                }
                            } else {
                                for _ in 0..-n {
                                    self.stack.pop();
                                }
                            }
                        }
                    }
                    61 => { // =
                        let mut command = split_string(self.read_string()?)?;
                        if command.len() > 0 {
                            match Command::new(command.remove(0)).args(command).output() {
                                Ok(output) => {
                                    funge.output.push(join(&output.stdout.into_iter().map(|i| chr(i)).collect::<Result<Vec<char>>>()?, ""))?;
                                    self.stack.push(match output.status.code() {
                                        Some(i) => cast_int(i)?,
                                        None => I::zero()
                                    });
                                }
                                Err(_) => self.stack.push(I::one())
                            }
                        } else {
                            self.stack.push(I::one());
                        }
                    }
                    40 => { // ( no fingerprints are implemented
                        self.read_fingerprint()?;
                        // self.fingerprint_ops[] = self.Reverse;
                        self.reflect();
                    }
                    41 => { // )
                        self.read_fingerprint()?;
                        // self.fingerprint_ops.pop()
                        self.reflect();
                    }
                    105 => { // i
                        let file = self.read_string()?;
                        let flags = self.stack.pop();
                        let y0 = cast_int(self.stack.pop())?;
                        let x0 = cast_int(self.stack.pop())?;
                        match read_file(&file) {
                            Ok(text) => {
                                let (width, height) = if flags.is_odd() {  // binary mode
                                    let code: Vec<char> = text.chars().collect();
                                    funge.insert_code(vec![join(&code, "")], x0, y0)?;
                                    (text.len(), 1)
                                } else {
                                    let text: Vec<&str> = text.lines().collect();
                                    let height = text.len();
                                    let width = text.iter().map(|i| i.len()).max().or(Some(0)).unwrap();
                                    let mut code: Vec<String> = Vec::new();
                                    for line in text {
                                        code.push(line.to_string());
                                    }
                                    funge.insert_code(code, x0, y0)?;
                                    (width, height)
                                };
                                self.stack.push(cast_int(width)?);
                                self.stack.push(cast_int(height)?);
                                self.stack.push(cast_int(x0)?);
                                self.stack.push(cast_int(y0)?);
                            }
                            _ => self.reflect()
                        }

                    }
                    106 => { // j
                        let n: isize = cast_int(self.stack.pop())?;
                        if n < 0 {
                            self.delta = self.delta.iter().map(|i| -i).collect();
                        }
                        for _ in 0..n.abs() {
                            self.movep(&funge);
                        }
                        if n < 0 {
                            self.delta = self.delta.iter().map(|i| -i).collect();
                        }
                        return Ok((funge, vec![self], true))
                    }
                    107 => { // k
                        let n: isize = cast_int(self.stack.pop())?;
                        if n == 0 { // special case
                            self.movep(&funge);
                            return Ok((funge, vec![self], true))
                        } else {
                            let k_op = self.next_op(&funge)?;
                            let mut ips = vec![self];
                            let mut advance = true;
                            for _ in 0..n {
                                let mut new_ips = Vec::new();
                                let n_ips = ips.len();
                                for ip in ips {
                                    funge = {
                                        let (f, ips, adv) = ip.exe(funge, k_op, n_ips)?;
                                        advance = adv;
                                        new_ips.extend(ips);
                                        f
                                    };
                                }
                                ips = new_ips;
                            }
                            return Ok((funge, ips, advance))
                        }
                    }
                    110 => self.stack.clear(), // n
                    111 => { // o
                        let file = self.read_string()?;
                        let flags = self.stack.pop();
                        let y0 = cast_int(self.stack.pop())?;
                        let x0 = cast_int(self.stack.pop())?;
                        let height: isize = cast_int(self.stack.pop())?;
                        let width: isize = cast_int(self.stack.pop())?;
                        let mut text = Vec::new();
                        if flags.is_odd() { // linear mode
                            for y in y0..y0 + height {
                                let mut line = String::new();
                                for x in x0..x0 + width {
                                    line.push(chr(funge.code[&vec![x, y]])?);
                                }
                                line = line.lines().map(|l| l.trim_end().to_string() + "\n").collect();
                                line = line.trim_end().to_string();
                                text.push(line);
                            }
                        } else {
                            for y in y0..y0 + height {
                                let mut line = String::new();
                                for x in x0..x0 + width {
                                    line.push(chr(funge.code[&vec![x, y]])?);
                                }
                                text.push(line);
                            }
                        }
                        let mut text = join(&text, "\n");
                        text.push_str("\n");
                        if let Err(_) = fs::write(file, text) {
                            self.reflect();
                        }
                    }
                    113 => {
                        Err(Error::new(FungeError::Quit(cast_int(self.stack.pop())?)))?;
                    } // q
                    114 => self.reflect(), // r
                    115 => { // s
                        self.movep(&funge);
                        funge.insert(self.stack.pop(), vec![self.position[0], self.position[1]]);
                    }
                    116 => { // t
                        let mut new = self.split(n_ips);
                        new.reflect();
                        new_ips.push(new);
                    }
                    117 => { // u
                        if self.stack.len_stack() <= 1 {
                            self.reflect();
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
                        let n: isize = cast_int(self.stack.pop())?;
                        funge.shrink_extent();
                        let counter = self.get_info(&funge)?;
                        if n > 0 {
                            let n = n as usize;
                            let l = self.stack.len();
                            let tmp = if n > l {
                                I::zero()
                            } else {
                                self.stack[self.stack.len_stack() - 1][l - n].to_owned()
                            };
                            for _ in 0..counter {
                                self.stack.pop();
                            }
                            self.stack.push(tmp);
                        }
                    }
                    122 => {} // z
                    48..=57 => self.stack.push(op - cast_int(48)?), // 0123456789
                    97..=102 => self.stack.push(op - cast_int(87)?), // abcdef
                    _ => self.not_implemented(&funge)?
                }
            } else {
                self.not_implemented(&funge)?;
            }
        } else {
            self.not_implemented(&funge)?;
        }
        // let mut ips = Vec::new();
        // ips.extend(new_ips);
        new_ips.push(self);
        Ok((funge, new_ips, false))
    }
}


#[derive(Clone)]
struct Rules {
    instruction_set: Vec<u8>,
    on_error: OnError
}

impl Rules {
    fn new() -> Result<Self> {
        Ok(Self {
            instruction_set: Self::get_instruction_set("B98")?,
            on_error: Self::get_on_error("Reflect")?
        })
    }

    fn with_rules<T: ToString>(version: T) -> Result<Self> {
        Ok(Self {
            instruction_set: Self::get_instruction_set(version.to_string())?,
            on_error: Self::get_on_error(version)?
        })
    }

    fn get_instruction_set<T: ToString>(version: T) -> Result<Vec<u8>> {
        Ok(match &version.to_string().to_uppercase()[..] {
            "B93" => "!\"#$%&*+,-./0123456789:<>?@\\^_`gpv|~",
            "B97" => "!\"#$%&\'*+,-./0123456789:<>?@\\^_`gpv|~abcdefg",
            "B98" => "!\"#$%&\'()*+,-./0123456789:;<=>?@[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~",
            _ => Err(Error::new(FungeError::Version(version.to_string())))?
        }.chars().map(|i| ord(i).unwrap()).collect())
    }

    fn get_on_error<T: ToString>(action: T) -> Result<OnError> {
        match OnError::from_str(&action.to_string()) {
            Ok(on_error) => Ok(on_error),
            _ => Ok(match &action.to_string().to_uppercase()[..] {
                    "B93" => Self::get_on_error("Ignore")?,
                    "B97" => Self::get_on_error("Ignore")?,
                    "B98" => Self::get_on_error("Reflect")?,
                    _ => Err(Error::new(FungeError::Version(action.to_string())))?
                })
        }
    }
}


#[derive(Clone, EnumString)]
enum OnError {
    Ignore,
    Reflect,
    #[allow(dead_code)]
    Quit,
}


fn read_file(file: &String) -> Result<String> {
    Ok(join(&fs::read(file)?.iter().map(|i| chr(*i)).collect::<Result<Vec<char>>>()?, ""))
}


#[derive(Clone)]
pub struct Rect {
    pub left: isize,
    pub right: isize,
    pub top: isize,
    pub bottom: isize
}

impl Rect {
    pub fn new(left: isize, right: isize, top: isize, bottom: isize) -> Self {
        Self { left, right, top, bottom }
    }

    pub fn width(&self) -> isize {
        self.right - self.left
    }

    pub fn height(&self) -> isize {
        self.bottom - self.top
    }

    pub fn contains(&self, pos: &Vec<isize>) -> bool {
        (self.left <= pos[0]) & (pos[0] < self.right) & (self.top <= pos[1]) & (pos[1] < self.bottom)
    }
}


#[derive(Clone)]
pub struct FungeSpace<I: Int> {
    pub orig_code: Vec<Vec<I>>,
    pub orig_rect: Rect,
    pub new_code: HashMap<Vec<isize>, I>,
    space: I
}

impl<I: Int> FungeSpace<I> {
    fn new(code: Vec<String>) -> Result<Self> {
        let code = code.into_iter().map(|line| line.replace(chr(12).unwrap(), "")).collect::<Vec<String>>();
        let mut new = Self {
            orig_code: Vec::new(),
            orig_rect: Rect::new(
                0,code.iter().map(|line| line.len()).max().or(Some(0)).unwrap() as isize,
                0,code.len() as isize
            ),
            new_code: HashMap::new(),
            space: cast_int(32)?
        };
        let width = new.orig_rect.width() as usize;
        for line in code {
            let mut i = line.chars().map(|c| ord(c)).collect::<Result<Vec<I>>>()?;
            i.extend(vec![new.space; &width - i.len()]);
            new.orig_code.push(i);
        }
        Ok(new)
    }

    pub fn insert(&mut self, index: Vec<isize>, op: I) {
        if self.orig_rect.contains(&index) {
            self.orig_code[index[1] as usize][index[0] as usize] = op;
        } else if op == self.space {
            self.new_code.remove(&index);
        } else {
            self.new_code.insert(index, op);
        }
    }

    pub fn get_string(&self, rect: Rect) -> Vec<String> {
        let mut string = Vec::new();
        for y in rect.top..rect.bottom {
            let mut line = Vec::new();
            if (self.orig_rect.top <= y) & (y < self.orig_rect.bottom) {
                for x in rect.left..0 {
                    line.push(*self.new_code.get(&vec![x, y]).unwrap_or(&self.space));
                }
                let left = max(self.orig_rect.left, rect.left) as usize;
                let right = min(self.orig_rect.right, rect.right) as usize;
                let orig_line = self.orig_code[y as usize][left..right].to_vec();
                let width = orig_line.len();
                line.extend(orig_line);
                if rect.right > self.orig_rect.right {
                    line.extend(vec![self.space; (self.orig_rect.right as usize) - width]);
                    for x in self.orig_rect.right..rect.right {
                        line.push(*self.new_code.get(&vec![x, y]).unwrap_or(&self.space));
                    }
                }
            } else {
                for x in rect.left..rect.right {
                    line.push(*self.new_code.get(&vec![x, y]).unwrap_or(&self.space));
                }
            }
            string.push(vec_to_string(line));
        }
        string
    }
}

impl<I: Int> Index<&Vec<isize>> for FungeSpace<I> {
    type Output = I;

    fn index(&self, index: &Vec<isize>) -> &Self::Output {
        if self.orig_rect.contains(index) {
            &self.orig_code[index[1] as usize][index[0] as usize]
        } else {
            self.new_code.get(index).unwrap_or(&self.space)
        }
    }
}


#[derive(Clone)]
pub struct Funge<I: Int> {
    pub extent: Rect,
    pub code: FungeSpace<I>,
    rules: Rules,
    pub steps: isize,
    pub ips: Vec<IP<I>>,
    pub input: IO,
    pub output: IO,
}

impl<I: Int> Funge<I> {
    pub fn new<T: ToString>(code: T) -> Result<Self> {
        let mut code: Vec<String> = code.to_string().lines().map(|i| String::from(i)).collect();
        let exe = env::current_exe()?.file_name().ok_or(Error::msg("No exe name"))?.to_str().unwrap().to_string();
        if code[0].starts_with(&*format!(r"#!/usr/bin/env {}", exe)) | code[0].starts_with(&*format!(r"#!/usr/bin/env -S {}", exe)) {
            code.remove(0);
        }
        let funge_space = FungeSpace::new(code)?;
        let mut new = Self {
            extent: funge_space.orig_rect.clone(),
            code: funge_space,
            rules: Rules::new()?,
            steps: 0,
            ips: Vec::new(),
            input: IO::new(),
            output: IO::new()
        };
        new.ips.push(IP::new(&new)?);
        Ok(new)
    }

    pub fn from_file(file: &String) -> Result<Self> {
        Ok(Self::new(read_file(file)?)?)
    }

    pub fn with_version<T: ToString>(mut self, version: T) -> Result<Self> {
        self.rules = Rules::with_rules(version)?;
        Ok(self)
    }

    pub fn with_arguments(mut self, args: Vec<String>) -> Self {
        self.input = IO::new().with_store(args);
        self
    }

    pub fn with_input(mut self, input: IO) -> Self {
        self.input = input;
        self
    }

    pub fn with_output(mut self, output: IO) -> Self {
        self.output = output;
        self
    }

    fn shrink_extent(&mut self) {
        let space = cast_int(32).expect("space");
        'left: for x in self.extent.left..self.extent.right {
            for y in self.extent.top..self.extent.bottom {
                if self.code[&vec![x, y]] != space {
                    self.extent.left = x;
                    break 'left
                }
            }
        }
        'right: for x in (self.extent.left..self.extent.right).rev() {
            for y in self.extent.top..self.extent.bottom {
                if self.code[&vec![x, y]] != space {
                    self.extent.right = x + 1;
                    break 'right
                }
            }
        }
        'top: for y in self.extent.top..self.extent.bottom {
            for x in self.extent.left..self.extent.right {
                if self.code[&vec![x, y]] != space {
                    self.extent.top = y;
                    break 'top
                }
            }
        }
        'bottom: for y in (self.extent.top..self.extent.bottom).rev() {
            for x in self.extent.left..self.extent.right {
                if self.code[&vec![x, y]] != space {
                    self.extent.bottom = y + 1;
                    break 'bottom
                }
            }
        }
    }

    fn grow_extent(&mut self, position: Vec<isize>) {
        if position[0] < self.extent.left {
            self.extent.left = position[0];
        } else if position[0] >= self.extent.right {
            self.extent.right = position[0] + 1;
        }
        if position[1] < self.extent.top {
            self.extent.top = position[1];
        } else if position[1] >= self.extent.bottom {
            self.extent.bottom = position[1] + 1;
        }
    }

    fn insert(&mut self, op: I, position: Vec<isize>) {
        self.code.insert(position.to_owned(), op);
        if let Ok(32) = cast_int(op) {
            self.shrink_extent();
        } else {
            self.grow_extent(position);
        }
    }

    fn insert_code(&mut self, code: Vec<String>, x0: isize, y0: isize) -> Result<()> {
        for (y, line) in code.iter().enumerate() {
            for (x, char) in line.chars().enumerate() {
                if char != ' ' {
                    let x1: isize = x.try_into()?;
                    let y1: isize = y.try_into()?;
                    self.insert(ord(char)?, vec![x0 + x1, y0 + y1]);
                }
            }
        }
        Ok(())
    }

    pub fn run(mut self) -> Result<i32> {
        loop {
            self = match self.step() {
                Err(error) => {
                    let error = error.downcast::<FungeError>()?;
                    match error {
                        FungeError::Quit(return_code) => return Ok(return_code),
                        error => Err(Error::new(error))?
                    }
                }
                Ok(funge) => funge
            }
        }
    }

    pub fn step(mut self) -> Result<Self> {
        self.ips.reverse();
        let mut new_ips = Vec::new();
        let n_ips = self.ips.len();
        for _ in 0..self.ips.len() {
            if let Some(ip) = self.ips.pop() {
                self = match ip.step(self, n_ips)? {
                    (f, ips) => {
                        new_ips.extend(ips);
                        f
                    }
                }
            }
        }
        self.ips.extend(new_ips);
        self.steps += 1;
        if self.ips.len() == 0 {
            Err(Error::new(FungeError::Quit(0)))
        } else {
            Ok(self)
        }
    }

    pub fn ips_pos(&self) -> Vec<Vec<isize>> {
        let mut pos = Vec::new();
        for ip in self.ips.iter() {
            pos.push(ip.position.to_owned());
        }
        pos
    }

    pub fn get_stack_string(&self) -> String {
        join(&(&self.ips).iter().map(|ip| ip.stack.to_string()).collect(), "\n")
    }
}