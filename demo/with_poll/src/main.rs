#[macro_use]
extern crate clap;
extern crate fanotify;
extern crate nix;

use fanotify::high_level::*;
use nix::poll::{poll, PollFd, PollFlags};
use std::process::Command;
use which::which;

fn main() {
    let app = clap_app!(fanotify_demo =>
        (version:       crate_version!())
        (author:        crate_authors!())
        (about:         crate_description!())
        (@arg verbose: -v --verbose "verbose")
        (@arg path: +required "watch target mount point")
        (@arg scanner: "scanner (if scanner exit by 0 then allow execute.)")
    )
    .get_matches();

    let fd = Fanotify::new_with_nonblocking(FanotifyMode::CONTENT);
    if let Err(e) = fd.add_mountpoint(
        FAN_OPEN_EXEC_PERM, //| FAN_CLOSE_WRITE,
        app.value_of("path").unwrap(),
    ) {
        eprintln!("Error on add_mountpoint: {}", e);
        std::process::exit(1);
    }

    let mut fds = [PollFd::new(fd.as_raw_fd(), PollFlags::POLLIN)];
    loop {
        let poll_num = poll(&mut fds, -1).unwrap();
        if poll_num > 0 {
            assert!(fds[0].revents().unwrap().contains(PollFlags::POLLIN));
            for event in fd.read_event() {
                if app.is_present("verbose") {
                    println!("{:#?}", event);
                }
                if event.events.contains(&FanEvent::OpenExecPerm) {
                    let mut response = FanotifyResponse::Allow;
                    if let Some(scanner) = app.value_of("scanner") {
                        let scanner = which(scanner).unwrap_or_else(|e|{
                            eprintln!("{}", e);
                            std::process::exit(1);
                        });
                        if event.path != scanner.to_str().unwrap() {
                            if Command::new(scanner)
                                .arg(event.path)
                                .status()
                                .unwrap()
                                .code()
                                .unwrap()
                                != 0
                            {
                                response = FanotifyResponse::Deny;
                            }
                        }
                    }
                    fd.send_response(event.fd, response);
                }
            }
        } else {
            eprintln!("poll_num <= 0!");
            break;
        }
    }
}
