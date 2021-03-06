use crate::opcode::*;
use colored::*;
use std::collections::HashMap;
use std::fmt::Debug;
use std::io;
use std::io::Write;

pub type Int = i64;

pub struct Program<S: ProgSender, R: ProgReceiver> {
    mem: Vec<Int>,
    aux_mem: HashMap<usize, Int>, // holds whatever does not fit in mem
    pointer: usize,
    input: R,
    output: S,
    rel_base: Int,
}

pub trait ProgSender: Debug {
    fn put(&mut self, num: Int);
}

pub trait ProgReceiver: Debug {
    fn get(&mut self) -> Option<Int>;
}

impl<S: ProgSender, R: ProgReceiver> Program<S, R> {
    pub fn new(data: &Vec<Int>, input: R, output: S) -> Self {
        let mut mem = data.clone();
        Program {
            mem,
            pointer: 0,
            input,
            output,
            rel_base: 0,
            aux_mem: HashMap::new(),
        }
    }
    /// Dispatchs the corresponding operation and returns the new pointer
    fn execute(&mut self, code: Opcode) {
        match code {
            Opcode::Add(m0, m1, m2) => self.add(m0, m1, m2),
            Opcode::Multiply(m0, m1, m2) => self.multiply(m0, m1, m2),
            Opcode::Input(m0) => self.input(m0),
            Opcode::Output(m0) => self.output(m0),
            Opcode::Halt => self.halt(),
            Opcode::Equals(m0, m1, m2) => self.equals(m0, m1, m2),
            Opcode::JumpIfTrue(m0, m1) => self.jump_if_true(m0, m1),
            Opcode::JumpIfFalse(m0, m1) => self.jump_if_false(m0, m1),
            Opcode::LessThan(m0, m1, m2) => self.less_than(m0, m1, m2),
            Opcode::SetRelBase(m0) => self.set_rel_base(m0),
        }
    }

    fn read(&self, p: usize) -> Int {
        if p < self.mem.len() {
            self.mem[p]
        } else {
            *self.aux_mem.get(&p).unwrap_or(&0)
        }
    }

    fn write(&mut self, p: usize, val: Int) {
        if p < self.mem.len() {
            self.mem[p] = val;
        } else {
            self.aux_mem.insert(p, val);
        }
    }

    fn get_relative_position(&self, offset_wrt_pointer: usize, m: Mode) -> usize {
        let literal_num = self.read(self.pointer + offset_wrt_pointer);
        match m {
            Mode::Inmediate => panic!("Inmediate mode in output arg WTF"),
            Mode::Position => literal_num as usize,
            Mode::Relative => (self.rel_base + literal_num) as usize,
        }
    }

    fn add(&mut self, m0: Mode, m1: Mode, m2: Mode) {
        let p = self.get_relative_position(3, m2);
        let val = self.get_param(1, m0) + self.get_param(2, m1);
        self.write(p, val);
        self.pointer += 4;
    }

    fn multiply(&mut self, m0: Mode, m1: Mode, m2: Mode) {
        let p = self.get_relative_position(3, m2);
        let val = self.get_param(1, m0) * self.get_param(2, m1);
        self.write(p, val);
        self.pointer += 4;
    }

    fn jump_if_true(&mut self, m0: Mode, m1: Mode) {
        self.pointer = match self.get_param(1, m0) != 0 {
            true => self.get_param(2, m1) as usize,
            false => self.pointer + 3,
        }
    }

    fn jump_if_false(&mut self, m0: Mode, m1: Mode) {
        self.pointer = match self.get_param(1, m0) == 0 {
            true => self.get_param(2, m1) as usize,
            false => self.pointer + 3,
        }
    }

    fn less_than(&mut self, m0: Mode, m1: Mode, m2: Mode) {
        let p = self.get_relative_position(3, m2);
        let val = (self.get_param(1, m0) < self.get_param(2, m1)) as Int;
        self.write(p, val);
        self.pointer += 4;
    }

    fn equals(&mut self, m0: Mode, m1: Mode, m2: Mode) {
        let p = self.get_relative_position(3, m2);
        let val = (self.get_param(1, m0) == self.get_param(2, m1)) as Int;
        self.write(p, val);
        self.pointer += 4;
    }

    fn set_rel_base(&mut self, m0: Mode) {
        let literal_num = self.read(self.pointer + 1);
        self.rel_base += match m0 {
            Mode::Inmediate => literal_num,
            Mode::Position => self.read(literal_num as usize),
            Mode::Relative => self.read((self.rel_base + literal_num) as usize),
        };
        self.pointer += 2;
    }

    fn input(&mut self, m0: Mode) {
        // Get the input num from the input field or stdin oherwise
        let n: Int = match self.input.get() {
            Some(x) => x,
            None => {
                let mut inp = String::new();
                print!("Input please, human: ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut inp).unwrap();
                let nn = inp.trim().parse();
                if nn.is_err() {
                    self.input(m0);
                    return;
                }
                nn.unwrap()
            }
        };
        let p = self.get_relative_position(1, m0);
        self.write(p, n);
        self.pointer += 2;
    }

    fn output(&mut self, m0: Mode) {
        let out = self.get_param(1, m0);
        self.output.put(out);
        self.pointer += 2;
    }

    fn halt(&mut self) {}

    fn get_param(&mut self, position: usize, inmediate_mode: Mode) -> Int {
        let literal_num = self.read(self.pointer + position);
        match inmediate_mode {
            Mode::Inmediate => literal_num,
            Mode::Position => self.read(literal_num as usize),
            Mode::Relative => self.read((self.rel_base + literal_num) as usize),
        }
    }

    pub fn peak_input(&self) -> &R {
        &self.input
    }

    pub fn peak_output(&self) -> &S {
        &self.output
    }

    pub fn peak_mem(&self) -> &[Int] {
        &self.mem
    }

    fn debug(&self, last_code: Opcode) {
        let dbg = "[Debug] ".green();
        let mut c: char;
        let mut inp: String;
        while {
            print!(
                "{}lastop({:^24}) pointer({:^3}) $ ",
                dbg,
                format!("{:?}", last_code),
                self.pointer
            );
            io::stdout().flush().unwrap();
            inp = String::new();
            io::stdin().read_line(&mut inp).unwrap();
            c = inp.chars().next().unwrap();
            c != 'c'
        } {
            match c {
                'm' => {
                    let mut parts = inp.splitn(3, ' ');
                    parts.next();
                    if let Some(ini) = parts.next() {
                        if let Some(end) = parts.next() {
                            let x: usize = ini.parse().unwrap_or(0);
                            let y: usize = end.trim().parse().unwrap_or(self.mem.len() - 1);
                            println!("{}mem {}..={} {:?}", dbg, x, y, &self.mem[x..=y]);
                        } else {
                            println!("{}expected m x..=y", dbg);
                        }
                    } else {
                        println!("{}mem {:?}", dbg, self.mem);
                    }
                }
                'p' => println!("{}pointer {:?}", dbg, self.pointer),
                'i' => println!("{}input {:?}", dbg, self.input),
                'o' => println!("{}output {:?}", dbg, self.output),
                'b' => println!("{}rel_base {:?}", dbg, self.rel_base),
                _ => break,
            }
        }
    }

    pub fn run_debug_mode(&mut self) {
        let mut op: Opcode;
        let mut old_pointer;
        println!(
            "{}",
            "
            pick
              [c]     continue
              [m x y] view mem in range x..=y, ignore = view all
              [p]     view pointer
              [i]     view input stack
              [o]     view output stack
              [b]     view rel_base
             "
            .green()
        );

        while {
            old_pointer = self.pointer;
            op = from_num(self.mem[self.pointer]);
            self.execute(op.clone());
            old_pointer != self.pointer
        } {
            self.debug(op);
        }
    }

    pub fn run(&mut self) {
        let mut op: Opcode;
        let mut old_pointer;
        while {
            old_pointer = self.pointer;
            op = from_num(self.read(self.pointer));
            self.execute(op);
            old_pointer != self.pointer
        } {}
    }
}
