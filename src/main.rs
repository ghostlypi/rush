use std::{io, process::exit, env};
use nix::{sys::wait::waitpid,unistd::{fork, ForkResult, write}};
use exec;
use shell_words::{self, split};

enum Priority{
    BG,
    FG,
}
struct Command {
    cmd : String,
    priority : Priority,
    args : Vec<String>,
}

impl Command {
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

    fn process(&self){
        if self.cmd.as_str() == "exit" {
            exit(0);    
        } else if self.cmd.as_str() == "jobs" {
            // TODO: print jobs
        } else if self.cmd.as_str() == "kill" {
            // TODO: kill specified job
        } else {
            match unsafe{fork()} {
                Ok(ForkResult::Parent { child, .. }) => {
                    // println!("Continuing execution in parent process, new child has pid: {}", child);
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
}

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    loop{
        let raw_path = env::current_dir()?;
        let mut path = raw_path.as_os_str().to_str().unwrap().to_string();
        path.push_str("> ");
        let _err = write(libc::STDOUT_FILENO, path.as_str().as_bytes());
        io::stdin().read_line(&mut buffer)?;
        let split = split(&buffer);
        let tokens = match split{Ok(x) => x, Err(_e) => vec!["".to_string()]};
        let mut tokens = tokens.iter();
        let cmd = match tokens.nth(0) {
            None => "".to_string(),
            Some(x) => x.to_string(),
        };
        let context : Priority;
        let bg : &str = match tokens.nth_back(0) {
            None => "",
            Some(x) => x,
        };
        if bg == "&" {
            context = Priority::BG;
        } else {
           context = Priority::FG;
        }
        let mut command = Command {
            cmd: cmd,
            priority: context,
            args: tokens.map(|x| x.as_str().to_string()).collect()
        };
        if matches!(command.priority, Priority::FG) {
            command.args.push(bg.to_string())
        }
        command.print();
        command.process();
        buffer = String::new();
    } 
}