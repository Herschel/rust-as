use anyhow::{anyhow, Context, Result};
use pest::{
    iterators::{Pair, Pairs},
    Parser,
};
use swf::{Fixed8, SwfStr, Twips};
#[macro_use]
extern crate pest_derive;

#[derive(Parser)]
#[grammar = "rfl.pest"]
pub struct RflParser;

fn main() -> Result<()> {
    let filename = std::env::args()
        .nth(1)
        .ok_or(anyhow!("Missing input file"))?;
    let input = std::fs::read_to_string(filename).context("Unable to read input file")?;

    let pairs = RflParser::parse(Rule::swf, &input)?;

    //let mut buffer = vec![];
    let mut swf_job = SwfJob {
        name: "test.swf".to_string(),
        header: swf::Header {
            version: 32,
            compression: swf::Compression::None,
            stage_size: swf::Rectangle {
                x_min: Twips::ZERO,
                y_min: Twips::ZERO,
                x_max: Twips::from_pixels(550.0),
                y_max: Twips::from_pixels(400.0),
            },
            frame_rate: Fixed8::from_f32(30.0),
            num_frames: 0,
        },
        background_color: swf::Color::from_rgba(0xffffffff),
        file_attributes: swf::FileAttributes::default(),
        tags: vec![],
    };

    for pair in pairs {
        visit_swf(&mut swf_job, pair)?;
    }

    // Finalize SWF.
    swf_job
        .tags
        .insert(0, swf::Tag::FileAttributes(swf_job.file_attributes));
    swf_job
        .tags
        .insert(1, swf::Tag::SetBackgroundColor(swf_job.background_color));

    // Write SWF to file system.
    let mut output = vec![];
    swf::write_swf(&swf_job.header, &swf_job.tags, &mut output)
        .map_err(|_| anyhow!("Unable to write {}", swf_job.name))?;
    std::fs::write(swf_job.name, output)?;

    Ok(())
}

fn visit_swf<'a>(swf_job: &mut SwfJob<'a>, pair: Pair<'a, Rule>) -> Result<()> {
    let pairs = pair.into_inner();
    for pair in pairs {
        match pair.as_rule() {
            Rule::swf_name => swf_job.name = string_param(pair)?.to_string(),
            Rule::swf_dimensions => {
                let mut pairs = pair.into_inner();
                swf_job.header.stage_size = swf::Rectangle {
                    x_min: twips_param(pairs.next().unwrap())?,
                    y_min: twips_param(pairs.next().unwrap())?,
                    x_max: twips_param(pairs.next().unwrap())?,
                    y_max: twips_param(pairs.next().unwrap())?,
                }
            }
            Rule::swf_framerate => swf_job.header.frame_rate = Fixed8::from_f64(float_param(pair)?),
            Rule::swf_background_color => {
                swf_job.background_color =
                    swf::Color::from_rgb(u32::from_str_radix(pair.as_str(), 16)?, 255)
            }
            // Tags
            Rule::do_action => visit_do_action(swf_job, pair.into_inner())?,
            Rule::metadata => swf_job.tags.push(swf::Tag::Metadata(SwfStr::from_utf8_str(
                pair.into_inner().as_str(),
            ))),
            Rule::show_frame => {
                swf_job.header.num_frames += 1;
                swf_job.tags.push(swf::Tag::ShowFrame);
            }

            _ => return Err(anyhow!("Unexpected rule: {:?}", pair.as_rule())),
        }
    }
    Ok(())
}

fn visit_do_action<'a>(swf_job: &mut SwfJob<'a>, pairs: Pairs<Rule>) -> Result<()> {
    use swf::avm1::{types::Action, write::Writer};

    // let start = buffer.len();
    // let mut writer = Writer::new(buffer, swf_job.header.version);
    // for pair in pairs {
    //     let action = match pair.as_rule() {
    //         Rule::avm1_push => Action::Trace,
    //         Rule::avm1_trace => Action::Trace,
    //         _ => return Err(anyhow!("Unexpected rule: {:?}", pair.as_rule())),
    //     };
    //     writer.write_action(&action)?;
    // }
    Ok(())
}

fn string_param<'i>(pair: Pair<'i, Rule>) -> Result<&str> {
    match pair.into_inner().next() {
        Some(p) if p.as_rule() == Rule::string_literal => Ok(&p.as_str()[1..p.as_str().len() - 1]),
        _ => Err(anyhow!("Expected string literal")),
    }
}

fn float_param<'i>(pair: Pair<'i, Rule>) -> Result<f64> {
    match pair.into_inner().next() {
        Some(p) if p.as_rule() == Rule::float_literal => Ok(p.as_str().parse()?),
        _ => Err(anyhow!("Expected float literal")),
    }
}

fn twips_param<'i>(pair: Pair<'i, Rule>) -> Result<swf::Twips> {
    if pair.as_rule() == Rule::twips_literal {
        if let Some(twips) = pair.as_str().strip_suffix("twips") {
            Ok(Twips::new(twips.parse::<i32>()?))
        } else {
            Ok(Twips::from_pixels(pair.as_str().parse()?))
        }
    } else {
        Err(anyhow!("Expected twips literal"))
    }
}

struct SwfJob<'a> {
    name: String,
    file_attributes: swf::FileAttributes,
    background_color: swf::SetBackgroundColor,
    header: swf::Header,
    tags: Vec<swf::Tag<'a>>,
}
