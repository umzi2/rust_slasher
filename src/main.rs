mod slashers;

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use pepecore::enums::ImgColor;
use pepecore::read::read_in_path;
use pepecore_array::{ SVec, Shape};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::{fs, io};
use walkdir::WalkDir;
use crate::slashers::central_scan::slasher_central;
use crate::slashers::standard::slasher;

/// CLI-парсер
#[derive(Parser, Debug)]
#[command(
    name = "slasher-cli",
    about = "Нарезает изображения"
)]
struct Args {
    /// Входная папка с изображениями
    #[arg(short, long)]
    input: PathBuf,

    /// Папка для результатов (по умолчанию рядом с input)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Порог потока (thread)
    #[arg(short = 't', long, default_value_t = 0)]
    thread: u8,

    /// Высота кропа в пикселях
    #[arg(short = 'h', long, default_value_t = 15_000)]
    crop_height: usize,

    /// Отступ ауры в пикселях
    #[arg(short = 'a', long, default_value_t = 100)]
    aura_margin: usize,

    /// Шаг сканирования в пикселях
    #[arg(short = 's', long, default_value_t = 5)]
    scan_step: usize,

    #[arg(short = 'f', long, default_value_t = false)]
    folder_mode: bool,

    #[arg(short = 'c', long, default_value_t = false)]
    central_scan: bool,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    let input_root = args.input.canonicalize()?;
    let output_root = args.output.unwrap_or_else(|| input_root.clone());
    fs::create_dir_all(&output_root)?;

    let mut images_grouped: Vec<Vec<PathBuf>> = Vec::new();

    if args.folder_mode {
        let mut folder_map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();

        for entry in WalkDir::new(&input_root).into_iter().filter_map(Result::ok) {
            let p = entry.path();
            if p.is_file() {
                // Получаем путь относительно корня
                let rel_folder = p
                    .parent()
                    .unwrap()
                    .strip_prefix(&input_root)
                    .unwrap_or_else(|_| Path::new(""))
                    .to_path_buf();

                folder_map
                    .entry(rel_folder)
                    .or_default()
                    .push(p.to_path_buf());
            }
        }

        images_grouped = folder_map.into_values().collect();
    } else {
        for entry in WalkDir::new(&input_root).into_iter().filter_map(Result::ok) {
            let p = entry.path();
            if p.is_file() {
                images_grouped.push(vec![p.to_path_buf()]);
            }
        }
    }
    let pb = ProgressBar::new(images_grouped.len() as u64);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    let total_images = images_grouped.iter().map(|g| g.len()).sum::<usize>();
    let pb = ProgressBar::new(total_images as u64);
    pb.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} {msg}")
            .unwrap()
            .progress_chars("##-"),
    );

    for group in images_grouped {
        let mut image = read_in_path(&group[0], ImgColor::RGB)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))?;
        image.as_u8();
        let mut n = 1;
        let file_name = if group.len() > 1 {
            let (mut h, w, c) = image.shape.get_shape();
            let data = image
                .get_mut_vec::<u8>()
                .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))?;
            for path_index in 1..group.len() {
                let img_temp = read_in_path(&group[path_index], ImgColor::RGB)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))?;
                let t_h = img_temp.shape.get_height();
                data.extend(
                    img_temp
                        .get_data::<u8>()
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("{e:?}")))?,
                );
                h += t_h;
                n += 1
            }
            image.shape = Shape::new(h, w, c);

            group
                .first()
                .expect("хуя")
                .parent()
                .unwrap()
                .strip_prefix(&input_root)
                .unwrap_or_else(|_| Path::new(""))
                .to_path_buf()
                .display()
                .to_string()
        } else {
            group
                .first()
                .expect("хуя")
                .file_stem() // вернёт OsStr без расширения
                .unwrap_or_else(|| std::ffi::OsStr::new("unnamed"))
                .to_string_lossy()
                .into()
        };

        pb.set_message(file_name.clone());

        if let Err(e) = process_image(
            &file_name,
            &mut image,
            &output_root,
            args.thread,
            args.crop_height,
            args.aura_margin,
            args.scan_step,
            args.central_scan
        ) {
            eprintln!("Ошибка обработки {}: {}", &file_name, e);
        }

        pb.inc(n);
    }

    pb.finish_with_message("Готово!");
    Ok(())
}

fn process_image(
    file_name: &String,
    image: &mut SVec,
    output_root: &Path,
    thread: u8,
    crop_height: usize,
    aura_margin: usize,
    scan_step: usize,
    no_normal: bool,
) -> io::Result<()> {
    fs::create_dir_all(&output_root.join(file_name))?;
    if no_normal{
        slasher_central(
            image,
            &output_root.join(file_name),
            file_name,
            thread,
            crop_height,
            aura_margin,
            scan_step,
        );
    }
    else {
        slasher(
            image,
            &output_root.join(file_name),
            file_name,
            thread,
            crop_height,
            aura_margin,
            scan_step,
        );
    }
    Ok(())
}


