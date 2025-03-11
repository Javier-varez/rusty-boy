extern crate alloc;

use alloc::{
    string::{String, ToString},
    vec,
    vec::Vec,
};

use crankstart::{
    file::FileSystem,
    geometry::ScreenRect,
    graphics::{Font, LCDColor},
};

use crankstart_sys::{LCDSolidColor, PDButtons};
use euclid::{Point2D, Size2D};
use {
    crankstart::system::System,
    crankstart_sys::{LCD_COLUMNS, LCD_ROWS},
};

fn load_file(fs: &FileSystem, name: &str) -> Result<Rom, anyhow::Error> {
    let stat = fs.stat(name)?;
    let mut data = vec![0; stat.size as usize];
    let file = fs.open(name, crankstart_sys::FileOptions::kFileReadData)?;
    file.read(&mut data)?;
    Ok(Rom {
        file_name: name.to_string(),
        data,
    })
}

const PADDING: i32 = 8;
const BOX_HEIGHT: i32 = super::TEXT_HEIGHT + PADDING;
const DISP_WIDTH: i32 = LCD_COLUMNS as i32;
const DISP_HEIGHT: i32 = LCD_ROWS as i32;

const BOXES_PER_PAGE: usize = ((DISP_HEIGHT - 2 * PADDING) / BOX_HEIGHT) as usize;

pub struct GameSelector {
    index: usize,
    choices: Vec<String>,
}

pub struct Rom {
    pub file_name: String,
    pub data: Vec<u8>,
}

impl GameSelector {
    pub fn new(font: &Font) -> Result<Self, anyhow::Error> {
        let fs = FileSystem::get();

        let file = fs.open("readme.txt", crankstart_sys::FileOptions::kFileWrite)?;
        file.write("Put roms here".as_bytes())?;

        let mut choices = vec![];
        choices.shrink_to(0);
        for file in fs.listfiles("", false)? {
            if !file.ends_with(".gb") {
                continue;
            }
            choices.push(file);
        }

        let selector = Self { index: 0, choices };
        selector.draw_picker(font)?;
        Ok(selector)
    }

    pub fn draw_empty_picker(&self, font: &Font) -> Result<(), anyhow::Error> {
        let graphics = crankstart::graphics::Graphics::get();
        graphics.clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;

        let text = "No games could be found";
        let text_width = graphics.get_text_width(font, text, 0)?;

        graphics.set_draw_mode(crankstart_sys::LCDBitmapDrawMode::kDrawModeFillBlack)?;
        graphics.draw_text(
            text,
            Point2D::new(
                (DISP_WIDTH - text_width) / 2,
                (DISP_HEIGHT - super::TEXT_HEIGHT) / 2,
            ),
        )?;
        Ok(())
    }

    pub fn draw_picker(&self, font: &Font) -> Result<(), anyhow::Error> {
        if self.choices.is_empty() {
            return self.draw_empty_picker(font);
        }

        let graphics = crankstart::graphics::Graphics::get();
        graphics.clear(LCDColor::Solid(LCDSolidColor::kColorWhite))?;

        let mut y_text_offset = PADDING + (BOX_HEIGHT - super::TEXT_HEIGHT) / 2;
        let mut y_rect_offset = PADDING;

        let skip = if self.index >= BOXES_PER_PAGE {
            self.index - BOXES_PER_PAGE + 1
        } else {
            0
        };
        for (i, c) in self
            .choices
            .iter()
            .skip(skip)
            .take(BOXES_PER_PAGE)
            .enumerate()
        {
            graphics.fill_rect(
                ScreenRect::new(
                    Point2D::new(PADDING, y_rect_offset),
                    Size2D::new(DISP_WIDTH - 2 * PADDING, BOX_HEIGHT),
                ),
                LCDColor::Solid(
                    if (i == self.index)
                        || ((self.index >= BOXES_PER_PAGE) && (i == BOXES_PER_PAGE - 1))
                    {
                        LCDSolidColor::kColorBlack
                    } else {
                        LCDSolidColor::kColorWhite
                    },
                ),
            )?;

            graphics.set_draw_mode(crankstart_sys::LCDBitmapDrawMode::kDrawModeNXOR)?;
            graphics.draw_text(c, Point2D::new(PADDING * 2, y_text_offset))?;
            y_rect_offset += BOX_HEIGHT;
            y_text_offset += BOX_HEIGHT;
        }

        Ok(())
    }

    pub fn update(&mut self, font: &Font) -> Result<Option<Rom>, anyhow::Error> {
        let (_, pushed, _) = System::get().get_button_state()?;
        if (pushed & PDButtons::kButtonA).0 != 0 && !self.choices.is_empty() {
            let fs = FileSystem::get();
            return Ok(Some(load_file(&fs, &self.choices[self.index])?));
        }

        if (pushed & PDButtons::kButtonDown).0 != 0 && self.index < self.choices.len() - 1 {
            self.index += 1;
        }
        if (pushed & PDButtons::kButtonUp).0 != 0 && self.index > 0 {
            self.index -= 1;
        }

        self.draw_picker(font)?;
        Ok(None)
    }
}
