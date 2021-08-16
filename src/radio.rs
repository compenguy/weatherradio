use anyhow::Result;
use std::convert::TryFrom;
use std::io::BufRead;

pub(crate) struct RTL433;

pub(crate) struct Weather<R> {
    _child: std::process::Child,
    stdout: Option<std::io::BufReader<std::process::ChildStdout>>,
    _stderr: Option<std::io::BufReader<std::process::ChildStderr>>,
    channel_type: std::marker::PhantomData<R>,
}

impl Weather<RTL433> {
    pub(crate) fn new<P: AsRef<std::path::Path>>(binpath: P) -> Result<Self> {
        let mut child = std::process::Command::new(binpath.as_ref().as_os_str())
            .arg("-Mutc")
            .arg("-Fjson")
            .arg("-f915M")
            .arg("-R113")
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .spawn()?;

        let stdout = child.stdout.take().map(std::io::BufReader::new);
        let stderr = child.stderr.take().map(std::io::BufReader::new);
        Ok(Weather {
            _child: child,
            stdout,
            _stderr: stderr,
            channel_type: std::marker::PhantomData,
        })
    }

    pub(crate) fn get_line(&mut self) -> Option<String> {
        if let Some(stdout) = &mut self.stdout {
            let mut line = String::new();
            while line.is_empty() {
                let result = stdout.read_line(&mut line);
                log::trace!("Reading from rtl_433: {:?} => '{}'", result, line);
                match result {
                    Ok(0) => return None,
                    Ok(_) => return Some(line),
                    Err(_) => (),
                }
                log::error!("Error reading from rtl_433: {:?}", result);
            }
            unreachable!();
        } else {
            log::error!("No output pipe for rtl_433 process!");
            None
        }
    }
}

impl Iterator for Weather<RTL433> {
    type Item = crate::measurement::Record;

    fn next(&mut self) -> Option<Self::Item> {
        // retry getting lines and parsing them as json until we get one that
        // parses correctly, or until we reach the end of child process
        loop {
            let line = match self.get_line() {
                None => return None,
                Some(l) => l,
            };
            let json_result: std::result::Result<serde_json::Value, serde_json::Error> = serde_json::from_str(&line);
            let record_result: Result<crate::measurement::Record> = json_result.map_err(|e| e.into()).and_then(|j| crate::measurement::Record::try_from(j).map_err(|e| e.into()));
            if let Ok(record) = record_result {
                return Some(record);
            }
            log::error!("Error parsing rtl_433 output: {:?}", record_result);
        }
        /*
        if let Ok(Some(status)) = self.child.try_wait() {
            return None;
        }
        */
    }
}

// TODO: implement iter and stream for Weather<RTL433>
