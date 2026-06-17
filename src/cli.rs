use clap::builder::styling::{Color, Effects, RgbColor, Style, Styles};
use clap::{Parser, Subcommand};

const STYLES: Styles = Styles::styled()
    .header(
        Style::new()
            .fg_color(Some(Color::Rgb(RgbColor(212, 163, 104))))
            .effects(Effects::BOLD),
    )
    .usage(
        Style::new()
            .fg_color(Some(Color::Rgb(RgbColor(212, 163, 104))))
            .effects(Effects::BOLD),
    )
    .literal(Style::new().fg_color(Some(Color::Rgb(RgbColor(239, 228, 210)))))
    .placeholder(Style::new().fg_color(Some(Color::Rgb(RgbColor(160, 141, 118)))))
    .error(
        Style::new()
            .fg_color(Some(Color::Rgb(RgbColor(224, 139, 111))))
            .effects(Effects::BOLD),
    );

#[derive(Parser)]
#[command(version, styles = STYLES)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Subcommand)]
pub enum Command {
    Show { oid: String },
}
