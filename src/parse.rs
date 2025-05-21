// SPDX-FileCopyrightText: 2024 Luflosi <zonegen@luflosi.de>
// SPDX-License-Identifier: GPL-3.0-only

use nom::bytes::complete::take_while1;
use nom::error::context;
use nom::lib::std::result::Result::Err;
use nom::{branch::alt, bytes::complete::tag, AsChar, Err as NomErr, IResult, Parser};
use nom_language::error::{VerboseError, VerboseErrorKind};

type Res<T, U> = IResult<T, U, VerboseError<T>>;

#[derive(Debug, PartialEq, Eq)]
pub enum Command<'a> {
	Help,
	Send,
	Quit,
	Drop,
	Update(Update<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub enum Update<'a> {
	Add(Add<'a>),
	Delete(Delete<'a>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct Add<'a> {
	pub name: &'a str,
	pub ttl: u32,
	pub class: &'a str,
	pub type_: &'a str,
	pub data: &'a str,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Delete<'a> {
	pub name: &'a str,
	pub class: &'a str,
	pub type_: &'a str,
}

fn command(input: &str) -> Res<&str, Command> {
	context(
		"command",
		alt((help, send, quit, drop, update, add, delete)),
	)
	.parse(input)
}

fn help(input: &str) -> Res<&str, Command> {
	context("help", tag("help"))
		.parse(input)
		.map(|(next_input, _)| (next_input, Command::Help))
}

fn send(input: &str) -> Res<&str, Command> {
	context("send", tag("send"))
		.parse(input)
		.map(|(next_input, _)| (next_input, Command::Send))
}

fn quit(input: &str) -> Res<&str, Command> {
	context("quit", tag("quit"))
		.parse(input)
		.map(|(next_input, _)| (next_input, Command::Quit))
}

fn drop(input: &str) -> Res<&str, Command> {
	context("drop", tag("drop"))
		.parse(input)
		.map(|(next_input, _)| (next_input, Command::Drop))
}

fn update(input: &str) -> Res<&str, Command> {
	context("update", (tag("update"), tag(" "), add_or_delete))
		.parse(input)
		.map(|(next_input, (_, _, command))| (next_input, command))
}

fn ttl(input: &str) -> Res<&str, u32> {
	let digits = take_while1(AsChar::is_dec_digit);
	context("ttl", digits)
		.parse(input)
		.and_then(|(next_input, res)| {
			res.parse::<u32>().map_or_else(
				|_| Err(NomErr::Error(VerboseError { errors: vec![] })),
				|n| Ok((next_input, n)),
			)
		})
}

fn add(input: &str) -> Res<&str, Command> {
	let name =
		take_while1(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.');
	let class = take_while1(|c: char| c.is_ascii_uppercase());
	let type_ = take_while1(|c: char| c.is_ascii_uppercase());
	let data = take_while1(|c: char| c.is_ascii_graphic());
	context(
		"add",
		(
			tag("add"),
			tag(" "),
			name,
			tag(" "),
			ttl,
			tag(" "),
			class,
			tag(" "),
			type_,
			tag(" "),
			data,
		),
	)
	.parse(input)
	.map(
		|(next_input, (_, _, name, _, ttl, _, class, _, type_, _, data))| {
			(
				next_input,
				Command::Update(Update::Add(Add {
					name,
					ttl,
					class,
					type_,
					data,
				})),
			)
		},
	)
}

fn delete(input: &str) -> Res<&str, Command> {
	let name =
		take_while1(|c: char| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '.');
	let class = take_while1(|c: char| c.is_ascii_uppercase());
	let type_ = take_while1(|c: char| c.is_ascii_uppercase());
	context(
		"delete",
		(
			alt((tag("delete"), tag("del"))),
			tag(" "),
			name,
			tag(" "),
			class,
			tag(" "),
			type_,
		),
	)
	.parse(input)
	.map(|(next_input, (_, _, name, _, class, _, type_))| {
		(
			next_input,
			Command::Update(Update::Delete(Delete { name, class, type_ })),
		)
	})
}

fn add_or_delete(input: &str) -> Res<&str, Command> {
	context("add or delete", alt((add, delete))).parse(input)
}

pub fn parse(input: &str) -> Result<Command, NomErr<VerboseError<&str>>> {
	let res = command(input);
	match res {
		Ok(("", command)) => Ok(command),
		Ok(_) => Err(NomErr::Error(VerboseError {
			errors: vec![(
				input,
				VerboseErrorKind::Context("Trailing garbage after command"),
			)],
		})),
		Err(e) => Err(e),
	}
}

#[cfg(test)]
mod test {
	use super::{add, command, delete, parse, Add, Command, Delete, Update};
	use nom::error::ErrorKind;
	use nom::Err as NomErr;
	use nom_language::error::{VerboseError, VerboseErrorKind};

	#[test]
	fn command_token_test() {
		assert_eq!(command("help"), Ok(("", Command::Help)));
		assert_eq!(command("send"), Ok(("", Command::Send)));
		assert_eq!(command("quit"), Ok(("", Command::Quit)));
		assert_eq!(command("drop"), Ok(("", Command::Drop)));
		assert_eq!(
			command("update bla"),
			Err(NomErr::Error(VerboseError {
				errors: vec![
					("update bla", VerboseErrorKind::Nom(ErrorKind::Tag)),
					("update bla", VerboseErrorKind::Nom(ErrorKind::Alt)),
					("update bla", VerboseErrorKind::Context("delete")),
					("update bla", VerboseErrorKind::Nom(ErrorKind::Alt)),
					("update bla", VerboseErrorKind::Context("command")),
				]
			}))
		);
		assert_eq!(
			command("unknown"),
			Err(NomErr::Error(VerboseError {
				errors: vec![
					("unknown", VerboseErrorKind::Nom(ErrorKind::Tag)),
					("unknown", VerboseErrorKind::Nom(ErrorKind::Alt)),
					("unknown", VerboseErrorKind::Context("delete")),
					("unknown", VerboseErrorKind::Nom(ErrorKind::Alt)),
					("unknown", VerboseErrorKind::Context("command")),
				]
			}))
		);
	}

	#[test]
	fn add_test() {
		assert_eq!(
			add("add test.example.org. 300 IN AAAA ::1"),
			Ok((
				"",
				Command::Update(Update::Add(Add {
					name: "test.example.org.",
					ttl: 300,
					class: "IN",
					type_: "AAAA",
					data: "::1",
				}))
			))
		);
		assert_eq!(
			add("add test.example.org. f300 IN AAAA ::1"),
			Err(NomErr::Error(VerboseError {
				errors: vec![
					(
						"f300 IN AAAA ::1",
						VerboseErrorKind::Nom(ErrorKind::TakeWhile1),
					),
					("f300 IN AAAA ::1", VerboseErrorKind::Context("ttl")),
					(
						"add test.example.org. f300 IN AAAA ::1",
						VerboseErrorKind::Context("add"),
					),
				]
			}))
		);
	}

	#[test]
	fn delete_test() {
		assert_eq!(
			delete("del test.example.org. IN A"),
			Ok((
				"",
				Command::Update(Update::Delete(Delete {
					name: "test.example.org.",
					class: "IN",
					type_: "A",
				}))
			))
		);
		assert_eq!(
			delete("delete test.example.org. IN AAAA"),
			Ok((
				"",
				Command::Update(Update::Delete(Delete {
					name: "test.example.org.",
					class: "IN",
					type_: "AAAA",
				}))
			))
		);
		assert_eq!(
			add("dele test.example.org. IN A"),
			Err(NomErr::Error(VerboseError {
				errors: vec![
					(
						"dele test.example.org. IN A",
						VerboseErrorKind::Nom(ErrorKind::Tag)
					),
					(
						"dele test.example.org. IN A",
						VerboseErrorKind::Context("add")
					),
				]
			}))
		);
	}

	#[test]
	fn parse_test() {
		assert_eq!(parse("help"), Ok(Command::Help));
		assert_eq!(parse("send"), Ok(Command::Send));
		assert_eq!(parse("quit"), Ok(Command::Quit));
		assert_eq!(parse("drop"), Ok(Command::Drop));
		assert_eq!(
			parse("update delete example.org. IN A"),
			Ok(Command::Update(Update::Delete(Delete {
				name: "example.org.",
				class: "IN",
				type_: "A",
			})))
		);
		assert_eq!(
			parse("delete example.org. IN A"),
			Ok(Command::Update(Update::Delete(Delete {
				name: "example.org.",
				class: "IN",
				type_: "A",
			})))
		);
		assert_eq!(
			parse("add example.org. 123 IN A 1.2.3.4"),
			Ok(Command::Update(Update::Add(Add {
				name: "example.org.",
				ttl: 123,
				class: "IN",
				type_: "A",
				data: "1.2.3.4",
			})))
		);
		assert_eq!(
			parse("update delete example.org. IN A "),
			Err(NomErr::Error(VerboseError {
				errors: vec![(
					"update delete example.org. IN A ",
					VerboseErrorKind::Context("Trailing garbage after command")
				)]
			}))
		);
		assert_eq!(
			parse("help "),
			Err(NomErr::Error(VerboseError {
				errors: vec![(
					"help ",
					VerboseErrorKind::Context("Trailing garbage after command")
				)]
			}))
		);
	}
}
