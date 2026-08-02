use std::{
    io,
    process::{Child, Command, Stdio},
};

/// Run `commands` as a pipeline, connecting the stdout of each command to the stdin of the next one.
///
/// The stdin of the first command and the stdout of the last command are left untouched, so the caller can redirect them.
///
/// Unlike a shell pipeline, which only reports the exit status of the last command, a failing upstream command is also reported. Otherwise, for instance, a failing `tar` would produce an incomplete archive that looks successful.
pub fn execute_pipeline(commands: &mut [&mut Command], quiet: bool) -> io::Result<Option<i32>> {
    debug_assert!(!commands.is_empty());

    let last_index = commands.len() - 1;

    let mut children: Vec<Child> = Vec::with_capacity(commands.len());

    for (index, command) in commands.iter_mut().enumerate() {
        if index > 0 {
            // the previous command was spawned with a piped stdout
            let stdout = children[index - 1].stdout.take().unwrap();

            command.stdin(Stdio::from(stdout));
        }

        if index < last_index {
            command.stdout(Stdio::piped());
        }

        if quiet {
            command.stderr(Stdio::null());
        }

        match command.spawn() {
            Ok(child) => children.push(child),
            Err(error) => {
                kill_children(children);

                return Err(error);
            },
        }
    }

    let mut statuses = Vec::with_capacity(children.len());
    let mut first_error = None;

    // every child has to be reaped, even if one of them cannot be waited for
    for mut child in children {
        match child.wait() {
            Ok(status) => statuses.push(status),
            Err(error) => {
                if first_error.is_none() {
                    first_error = Some(error);
                }
            },
        }
    }

    if let Some(error) = first_error {
        return Err(error);
    }

    // the last command usually reports the most meaningful exit code, but an upstream failure must not be lost
    let last_code = statuses.pop().unwrap().code();

    if last_code != Some(0) {
        return Ok(last_code);
    }

    for status in statuses {
        let code = status.code();

        if code != Some(0) {
            return Ok(code);
        }
    }

    Ok(Some(0))
}

fn kill_children(children: Vec<Child>) {
    for mut child in children {
        let _ = child.kill();
        let _ = child.wait();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn successful_pipeline() {
        let mut command = Command::new("sh");
        command.args(["-c", "exit 0"]);

        assert_eq!(Some(0), execute_pipeline(&mut [&mut command], true).unwrap());
    }

    #[test]
    fn upstream_failure() {
        let mut command1 = Command::new("sh");
        command1.args(["-c", "exit 7"]);

        let mut command2 = Command::new("sh");
        command2.args(["-c", "cat >/dev/null"]);

        assert_eq!(Some(7), execute_pipeline(&mut [&mut command1, &mut command2], true).unwrap());
    }

    #[test]
    fn signal_without_exit_code() {
        let mut command = Command::new("sh");
        command.args(["-c", "kill -TERM $$"]);

        assert_eq!(None, execute_pipeline(&mut [&mut command], true).unwrap());
    }
}
