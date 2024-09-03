use crate::config::{Display, DISPLAY_HEIGHT, DISPLAY_WIDTH};
use core::fmt::Write;
use eg_seven_segment::SevenSegmentStyleBuilder;
use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, RgbColor, Size},
    primitives::{Line, PrimitiveStyle},
    text::Text,
    Drawable,
};
use heapless::{String, Vec};
use measurements::measurements::Data;
use u8g2_fonts::{
    fonts,
    types::{FontColor, VerticalPosition},
    FontRenderer,
};
// use ufmt::uwrite;
// use defmt::{info, println};

struct MainScreen {
    data: Vec<Data, 4>
}

pub fn draw_grid_main(display: &mut Display) {
    // Горизонтальні лінії
    let y = 13;
    Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
        .draw(display)
        .unwrap();

    let y = 13 + 28;
    Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
        .draw(display)
        .unwrap();

    let y = 13 + 28 * 2;
    Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
        .draw(display)
        .unwrap();

    let y = 13 + 28 * 3;
    Line::new(Point::new(0, y), Point::new(DISPLAY_WIDTH as i32 - 1, y))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
        .draw(display)
        .unwrap();

    // Вертикальні лінії
    let x = 32;
    Line::new(Point::new(x, 0), Point::new(x, DISPLAY_HEIGHT as i32 - 1))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
        .draw(display)
        .unwrap();

    let x = 85;
    Line::new(Point::new(x, 0), Point::new(x, DISPLAY_HEIGHT as i32 - 1))
        .into_styled(PrimitiveStyle::with_stroke(Rgb565::new(5, 10, 5), 1))
        .draw(display)
        .unwrap();
}

pub fn draw_labels_main(display: &mut Display) {
    let font = FontRenderer::new::<fonts::u8g2_font_profont17_mr>();

    font.render(
        "FAN",
        Point::new(3, 0),
        VerticalPosition::Top,
        FontColor::Transparent(Rgb565::WHITE),
        display,
    )
    .unwrap();

    font.render(
        "TEMP",
        Point::new(41, 0),
        VerticalPosition::Top,
        FontColor::Transparent(Rgb565::WHITE),
        display,
    )
    .unwrap();

    font.render(
        "RPM",
        Point::new(108, 0),
        VerticalPosition::Top,
        FontColor::Transparent(Rgb565::WHITE),
        display,
    )
    .unwrap();
}

pub fn draw_main(display: &mut Display, data: &Vec<Data, 4>) {
    let mut style_segment = SevenSegmentStyleBuilder::new()
        .digit_size(Size::new(15, 25)) // digits are 10x20 pixels
        .digit_spacing(2) // 5px spacing between digits
        .segment_width(3) // 5px wide segments
        // .segment_color(Rgb565::new(10, 20, 31))  // active segments are green
        .segment_color(Rgb565::RED) // active segments are green
        .inactive_segment_color(Rgb565::BLACK)
        .build();

    // Рядок перший. Номер
    style_segment.segment_color = Some(Rgb565::WHITE);
    Text::new("1", Point::new(8, 39), style_segment)
        .draw(display)
        .unwrap();

    // Рядок перший. Температура
    style_segment.segment_color = Some(Rgb565::RED);
    let mut text: String<2> = String::new();
    // uwrite!(&mut text, "{}", data[0].temp).unwrap();
    write!(text, "{:02}", data[0].temp).unwrap();
    Text::new(&text, Point::new(43, 39), style_segment)
        .draw(display)
        .unwrap();

    // Рядок перший. Оберти
    style_segment.segment_color = Some(Rgb565::BLUE);
    let mut text: String<4> = String::new();
    // uwrite!(&mut text, "{}", data[0].rpm).unwrap();
    write!(text, "{:04}", data[0].rpm).unwrap();
    Text::new(&text, Point::new(90, 39), style_segment)
        .draw(display)
        .unwrap();

    // Рядок другий. Номер
    style_segment.segment_color = Some(Rgb565::WHITE);
    Text::new("2", Point::new(8, 67), style_segment)
        .draw(display)
        .unwrap();

    // Рядок другий. Температура
    style_segment.segment_color = Some(Rgb565::RED);
    let mut text: String<2> = String::new();
    // uwrite!(&mut text, "{}", data[1].temp).unwrap();
    write!(text, "{:02}", data[1].temp).unwrap();
    Text::new(&text, Point::new(43, 67), style_segment)
        .draw(display)
        .unwrap();

    // Рядок другий. Оберти
    style_segment.segment_color = Some(Rgb565::BLUE);
    let mut text: String<4> = String::new();
    // uwrite!(&mut text, "{}", data[1].rpm).unwrap();
    write!(text, "{:04}", data[1].rpm).unwrap();
    Text::new(&text, Point::new(90, 67), style_segment)
        .draw(display)
        .unwrap();

    // Рядок третій. Номер
    style_segment.segment_color = Some(Rgb565::WHITE);
    Text::new("3", Point::new(8, 95), style_segment)
        .draw(display)
        .unwrap();

    // Рядок третій. Температура
    style_segment.segment_color = Some(Rgb565::RED);
    let mut text: String<2> = String::new();
    // uwrite!(&mut text, "{}", data[2].temp).unwrap();
    write!(text, "{:02}", data[2].temp).unwrap();
    Text::new(&text, Point::new(43, 95), style_segment)
        .draw(display)
        .unwrap();

    // Рядок третій. Оберти
    style_segment.segment_color = Some(Rgb565::BLUE);
    let mut text: String<4> = String::new();
    // uwrite!(&mut text, "{}", data[2].rpm).unwrap();
    write!(text, "{:04}", data[2].rpm).unwrap();
    Text::new(&text, Point::new(90, 95), style_segment)
        .draw(display)
        .unwrap();

    // Рядок четвертий. Номер
    style_segment.segment_color = Some(Rgb565::WHITE);
    Text::new("4", Point::new(8, 124), style_segment)
        .draw(display)
        .unwrap();

    // Рядок четвертий. Температура
    style_segment.segment_color = Some(Rgb565::RED);
    let mut text: String<2> = String::new();
    // uwrite!(&mut text, "{}", data[3].temp).unwrap();
    write!(text, "{:02}", data[3].temp).unwrap();
    Text::new(&text, Point::new(43, 124), style_segment)
        .draw(display)
        .unwrap();

    // Рядок четвертий. Оберти
    style_segment.segment_color = Some(Rgb565::BLUE);
    let mut text: String<4> = String::new();
    // uwrite!(&mut text, "{}", data[3].rpm).unwrap();
    write!(text, "{:04}", data[3].rpm).unwrap();
    Text::new(&text, Point::new(90, 124), style_segment)
        .draw(display)
        .unwrap();
}
