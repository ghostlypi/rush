use std::{io, process::exit, env, thread, ffi::{OsStr}};
use nix::{sys::{wait::waitpid, signal::{self,Signal}},unistd::{fork, ForkResult, write, Pid}};
use exec;
use shell_words::{self, split};
use signal_hook::{consts::{SIGINT}, iterator::Signals};

enum Priority{
    BG,
    FG,
}
struct Command {
    cmd : String,
    priority : Priority,
    args : Vec<String>,
}

struct Job {
    pid : i32,
    // jid : i32,
}

static mut FG : Job = Job {
    pid: 0,
};

impl Command {

    #[allow(dead_code)]
    fn print(&self){
        print!("Command {{");
        print!("{}, ", self.cmd);
        match self.priority {
            Priority::BG => print!("BG, "),
            Priority::FG => print!("FG, "),
        }
        print!("{:?}", self.args);
        print!("}}\n")
    }

    fn process(&mut self){
        if self.cmd.as_str() == "exit" {
            exit(0);    
        } else if self.cmd.as_str() == "jobs" {
            // TODO: print jobs
        } else if self.cmd.as_str() == "kill" {
            // TODO: kill specified job
        } else if self.cmd.as_str() == "clear"{
            if self.args.len() == 1 && match self.args.get(0){Some (x) => x, None => ""}.to_string().as_str() == ""{
               let _tmp = self.args.pop();
               self.args.push("-T".to_string());
               self.args.push(match env::var("TERM") {Ok(x) => x, Err(_e) => "".to_string(),});
            }
            self.execute();

        } else if self.cmd.as_str() == "ls" {
            if self.args.len() == 1 && match self.args.get(0){Some (x) => x, None => ""}.to_string().as_str() == ""{
                let _tmp = self.args.pop();
                self.args.push(".".to_string());
             }
             self.execute();
        } else if self.cmd.as_str() == "cd" {
            if self.args.len() == 1 && (match self.args.get(0){Some (x) => x, None => ""}.to_string().as_str() == "" || match self.args.get(0){Some (x) => x, None => ""}.to_string().as_str() == "~"){
                let _tmp = self.args.pop();
                self.args.push(match env::var("HOME") {Ok(x) => x, Err(_e) => "".to_string()});
            }
            let _res = env::set_current_dir::<String>(match self.args.pop() {Some (x) => x, None => "".to_string()});
        } else if matches!(&self.priority, Priority::FG){
            self.execute();
        }
    }

    fn execute(&self){
        match unsafe{fork()} {
            Ok(ForkResult::Parent { child, .. }) => {
                unsafe { 
                    FG = Job {
                        pid: child.as_raw(),
                    };
                }
                waitpid(child, None).unwrap();
            }
            Ok(ForkResult::Child) => {
                let _err = exec::Command::new(self.cmd.as_str())
                .args(&self.args)
                .exec();
            }
            Err(_) => println!("Fork failed"),
        }
    }
}

fn make_header() -> io::Result<()> {
    let raw_path = env::current_dir()?;
    let mut path = raw_path.as_os_str().to_str().unwrap().to_string();
    let home = match env::var("HOME") {Ok(x) => x, Err(_e)=> "".to_string()};
    let user = match env::var("USER") {Ok(x) => x, Err(_e)=> "".to_string()};
    let name = match gethostname::gethostname().to_str() {Some (x) => x.to_string(), None => "".to_string()};
    path = path.replace(home.as_str(),"~");
    let mut header = "\x1b[36m".to_string();
    header.push_str(user.as_str());
    header.push('@');
    header.push_str(name.as_str());
    header.push_str(":\x1b[0;31m");
    header.push_str("[\x1b[0;34m");
    header.push_str(path.as_str());
    header.push_str("\x1b[0;31m]\x1b[0;32m$");
    let _err = write(libc::STDOUT_FILENO, header.as_str().as_bytes());
    return Ok(());
}

fn main() -> io::Result<()> {
    //Set Shell
    let shell = env::current_exe();
    let shell = match shell {Ok(x) => x.as_os_str().to_owned(), Err(_e)=>OsStr::new("").to_owned()};
    env::set_var("SHELL", shell);

    //Initializing Signal handlers
    let mut signals = Signals::new(&[SIGINT])?;

    thread::spawn(move || {
        for sig in signals.forever() {
            if sig == 2 {
                if unsafe {FG.pid} != 0 {
                    signal::kill(Pid::from_raw(unsafe{FG.pid}), Signal::SIGINT).unwrap();
                }
            }
        }
    });

    //Initialize buffer
    let mut buffer = String::new();

    loop{
        //Get and write path to STDOUT
        let _res = make_header();

        //Read line
        io::stdin().read_line(&mut buffer)?;
        if buffer.len() == 0 {
            let _err = write(libc::STDOUT_FILENO, "\r".as_bytes());
            exit(0);
        }

        //Tokenize command
        let split = split(&buffer);
        let tokens = match split{Ok(x) => x, Err(_e) => vec!["".to_string()]};
        let mut tokens = tokens.iter();

        //Process the command
        let cmd = match tokens.nth(0) {
            None => "".to_string(),
            Some(x) => x.to_string(),
        };
        let context : Priority;
        let mut bg : String = match tokens.nth_back(0) {
            None => "".to_string(),
            Some(x) => x.to_string(),
        };
        if bg.as_str() == "&" {
            context = Priority::BG;
        } else {
           context = Priority::FG;
           bg = if bg.starts_with("$"){
            match env::var(bg.replace("$", "")) {
                Ok(x) =>  x,
                Err(_e) => "".to_string(),
            }
        } else {
            bg
        }
        }

        //Construct command object
        let mut command = Command {
            cmd: cmd,
            priority: context,
            args: tokens.map(|x| {
                if x.starts_with("$"){
                    match env::var(x.replace("$", "")) {
                        Ok(x) => x,
                        Err(_e) => "".to_string(),
                    }
                } else {
                    x.as_str().to_string()
                }
        }).collect()};

        //Re-add last arg if it's a background task
        if matches!(command.priority, Priority::FG) {
            command.args.push(bg.to_string())
        }

        //Do the actual work
        command.process();

        //Rest buffer
        buffer = String::new();
    } 
}