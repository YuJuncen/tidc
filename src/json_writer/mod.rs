use std::{io::{self, Write}};
use crate::parser::artifacts::*;

pub trait ToJSON {
    fn write_json_to<W: Write>(&self, w: W) -> io::Result<()>;
}

struct JsonObjectBuilder<W> {
    initial: bool,
    write: W,
}


impl<W:Write> JsonObjectBuilder<W> {
    fn on_writer(mut writer: W) -> io::Result<Self> {
        writer.write_all("{".as_bytes())?;
        Ok(JsonObjectBuilder {
            initial: true,
            write: writer
        })
    }

    fn write_key(&mut self, key: impl ToJSON) -> io::Result<()> {
        if !self.initial {
            self.write.write_all(",".as_bytes())?;
        }
        key.write_json_to(&mut self.write)?;
        self.write.write_all(":".as_bytes())?;
        self.initial = false;
        Ok(())
    }

    fn write_field(&mut self, key: impl ToJSON, value: impl ToJSON) -> io::Result<()> {
        self.write_key(key)?;
        value.write_json_to(&mut self.write)?;
        Ok(())
    }

    fn end(&mut self) -> io::Result<()> {
        self.write.write_all("}".as_bytes())
    }
}

impl<'a> ToJSON for &'a str {
    fn write_json_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_fmt(format_args!("{:?}", self))
    }
}

impl <'a> ToJSON for LogStr<'a> {
    fn write_json_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        match self {
            Self::Quoted(s) => w.write_all(s.as_bytes()),
            Self::Unquoted(s) => {
                w.write_all("\"".as_bytes())?;
                w.write_all(s.as_bytes())?;
                w.write_all("\"".as_bytes())
            }
        }
    }
}

impl <'a> ToJSON for LogLevel {
    fn write_json_to<W: Write>(&self, w: W) -> io::Result<()> {
        let desc = match self {
            LogLevel::Debug => "debug",
            LogLevel::Info => "info",
            LogLevel::Warn => "warn",
            LogLevel::Error => "error",
            LogLevel::Fatal => "fatal",
            LogLevel::Unknown => "<unknown>",
        };
        desc.write_json_to(w)
    }
}

impl <'a> ToJSON for FileLineRef<'a> {
    fn write_json_to<W: Write>(&self, w: W) -> io::Result<()> {
        let mut builder = JsonObjectBuilder::on_writer(w)?;
        builder.write_field("file", self.file)?;
        builder.write_field("line", self.line)?;
        builder.end()?;
        Ok(())
    }
}


impl <T: ToJSON> ToJSON for Option<T> {
    fn write_json_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        match self {
            None => w.write_all("null".as_bytes()),
            Some(item) => item.write_json_to(w)
        }
    }
}

impl <T: ToJSON> ToJSON for &T {
    fn write_json_to<W: Write>(&self, w: W) -> io::Result<()> {
        (*self).write_json_to(w)
    }
}

impl <'a> ToJSON for LogRecordRef<'a> {
    fn write_json_to<W: Write>(&self, w: W) -> io::Result<()> {
        let mut builder = JsonObjectBuilder::on_writer(w)?;
        builder.write_field("message", &self.message)?;
        builder.write_field("level", &self.level)?;
        builder.write_field("source", &self.source)?;
        builder.write_field("time", &self.time)?;
        builder.write_field("fields", self.entries.as_slice())?;
        builder.end()?;
        Ok(())
    }
}

impl <'a> ToJSON for &[LogFieldRef<'a>] {
    fn write_json_to<W: Write>(&self, w: W) -> io::Result<()> {
        let mut builder = JsonObjectBuilder::on_writer(w)?;
        for entry in self.iter() {
            builder.write_field(&entry.key, &entry.value)?;
        }
        builder.end()?;
        Ok(())
    }
}

impl <'a> ToJSON for TimeRef<'a> {
    fn write_json_to<W: Write>(&self, mut w: W) -> io::Result<()> {
        w.write_all(self.time_str.as_bytes())
    }
}