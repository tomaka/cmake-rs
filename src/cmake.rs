#![feature(plugin_registrar)]
#![feature(quote)]
#![feature(phase)]

#[phase(plugin)]
extern crate regex_macros;

extern crate regex;
extern crate rustc;
extern crate syntax;

use std::io::Reader;
use std::io::process::Command;
use std::path::Path;
use syntax::parse::token;
use syntax::ast::{ TokenTree };
use syntax::ext::base::{DummyResult, ExtCtxt, MacResult, MacItem};
use syntax::codemap::Span;

#[plugin_registrar]
#[doc(hidden)]
pub fn plugin_registrar(reg: &mut ::rustc::plugin::Registry) {
    reg.register_macro("cmake", macro_handler);
}

// main handler for the macro
fn macro_handler(ecx: &mut ExtCtxt, span: Span, token_tree: &[TokenTree]) -> Box<MacResult> {
    // getting the arguments from the macro
    let (srcPath, libname) = match parse_macro_arguments(ecx, token_tree) {
        Ok(t) => t,
        Err(msg) => {
            ecx.span_err(span, msg.as_slice());
            return DummyResult::any(span)
        }
    };

    // absolute path
    let srcPath = ::std::os::make_absolute(&srcPath);

    // creating the build directory
    let buildDirectory = srcPath.join("cmake-build");

    // creating the directory if it doesn't exist
    if !buildDirectory.exists() {
        match ::std::io::fs::mkdir_recursive(&buildDirectory, ::std::io::UserRWX) {
            Ok(_) => (),
            Err(err) => {
                ecx.span_err(span, format!("{}", err).as_slice());
                return DummyResult::any(span)
            }
        }
    }

    // getting the libraries output directory
    let outputDir = match ::std::os::homedir() {
        Some(d) => d.join(".cmake-rs-builds").join(ecx.ecfg.crate_id.name.clone()),
        None => {
            ecx.span_err(span, format!("unable to get your home directory").as_slice());
            return DummyResult::any(span)
        }
    };
    if !outputDir.exists() {
        match ::std::io::fs::mkdir_recursive(&outputDir, ::std::io::UserRWX) {
            Ok(_) => (),
            Err(err) => {
                ecx.span_err(span, format!("{}", err).as_slice());
                return DummyResult::any(span)
            }
        }
    }

    // invoking CMake
    let cmakeProcess = match Command::new("cmake")
                                .cwd(&buildDirectory)
                                .arg(format!("-DCMAKE_LIBRARY_OUTPUT_DIRECTORY:dir={}", outputDir.display()))
                                .arg(format!("-DCMAKE_ARCHIVE_OUTPUT_DIRECTORY:dir={}", outputDir.display()))
                                .arg(format!("{}", srcPath.display()))
                                .spawn()
    {
        Ok(p) => p,
        Err(err) => {
            ecx.span_err(span, format!("{}", err).as_slice());
            return DummyResult::any(span)
        }
    };

    // the content of stderr is converted into compilation errors
    match cmakeProcess.stderr.clone() {
        Some(mut stderr) => match stderr.read_to_str() {
            Ok(s) => if s.len() >= 1 {
                ecx.span_err(span, s.as_slice());
                return DummyResult::any(span)
            },
            Err(err) => ecx.span_err(span, format!("{}", err).as_slice())
        },
        None => ecx.span_warn(span, "could not open stderr pipe to cmake")
    }

    // invoking make
    let mut makeCommand = Command::new("make");
    makeCommand.cwd(&buildDirectory);
    match &libname { &Some(ref l) => { makeCommand.arg(l.as_slice()); }, _ => () };
    let makeProcess = match makeCommand.spawn() {
        Ok(p) => p,
        Err(err) => {
            ecx.span_err(span, format!("{}", err).as_slice());
            return DummyResult::any(span)
        }
    };

    // the content of stderr is converted into compilation errors
    match makeProcess.stderr.clone() {
        Some(mut stderr) => match stderr.read_to_str() {
            Ok(s) => if s.len() >= 1 {
                ecx.span_err(span, s.as_slice());
                return DummyResult::any(span)
            },
            Err(err) => ecx.span_err(span, format!("{}", err).as_slice())
        },
        None => ecx.span_warn(span, "could not open stderr pipe to cmake")
    }

    // outputting the library name
    {
        let link = match &libname {
            &Some(ref l) => format!("#[link_args = \"-L {}\"]\n#[link(name=\"{}\")]\nextern {{}}", outputDir.display(), l),
            &None => format!("")
        };
        str_to_item(ecx, link.as_slice())
    }
}

fn str_to_item(ecx: &mut ExtCtxt, content: &str) -> Box<MacResult> {
    let mut parser = ::syntax::parse::new_parser_from_source_str(ecx.parse_sess(), ecx.cfg(), "".to_string(), content.to_string());
    
    match parser.parse_item_with_outer_attributes() {
        None => fail!(),
        Some(i) => MacItem::new(i)
    }
}

// converts an Expr into a String if possible
fn expr_to_literal(expr: ::std::gc::Gc<::syntax::ast::Expr>)
    -> Result<String, String>
{
    Ok(match expr.node {
        syntax::ast::ExprLit(lit) => {
            match lit.node {
                syntax::ast::LitStr(ref s, _) => s.to_str(),
                _ => return Err(format!("expected string literal but got `{}`", syntax::print::pprust::lit_to_str(lit)))
            }
        }
        _ => return Err(format!("expected string literal but got `{}`", syntax::print::pprust::expr_to_str(expr)))
    })
}

// tries to obtain the macro arguments from the token tree
fn parse_macro_arguments(cx: &mut ExtCtxt, tts: &[syntax::ast::TokenTree])
    -> Result<(Path, Option<String>), String>
{
    let mut parser = ::syntax::parse::new_parser_from_tts(cx.parse_sess(), cx.cfg(), Vec::from_slice(tts));

    let path = match Path::new_opt(try!(expr_to_literal(cx.expand_expr(parser.parse_expr())))) {
        Some(p) => p,
        None => return Err(format!("the path is not utf-8!"))
    };

    let libname = if parser.eat(&token::COMMA) {
        Some(try!(expr_to_literal(cx.expand_expr(parser.parse_expr()))))
    } else { None };

    if !parser.eat(&token::EOF) {
        return Err("too many arguments".to_string());
    }

    Ok((path, libname))
}
