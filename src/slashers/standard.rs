use std::path::Path;
use pepecore::save::svec_save;
use pepecore_array::{ImgData, SVec, Shape};

pub fn slasher(
    img: &mut SVec,
    out_path: &Path,
    name: &str,
    threshold: u8,
    crop_height: usize,
    aura_margin: usize,
    scan_step: usize,
) {
    let (img_height, img_width, channels_opt) = img.shape.get_shape();
    let num_channels = channels_opt.unwrap();
    let pixel_data = img.get_data_mut::<u8>().unwrap();

    let mut next_crop_start = crop_height;
    let mut split_line = 0;

    let scan_range = (0..img_height - 1).step_by(scan_step);
    let mut crop_boundaries: Vec<usize> = vec![0];

    for row in scan_range {
        let row_start_idx = row * img_width;
        let next_row_start_idx = (row + 1) * img_width;
        let mut max_diff: u8 = 0;

        if row >= next_crop_start {
            let mut needs_split = true;
            let compare_base_idx = split_line * img_width * num_channels;
            if aura_margin + split_line < img_height {
                for offset in 1..aura_margin {
                    let mut local_max = 0;

                    for x in 0..img_width {
                        let ref_idx = ((split_line + offset) * img_width + x) * num_channels;
                        for ch in 0..num_channels {
                            local_max = local_max.max(
                                (pixel_data[compare_base_idx + ch] as i16
                                    - pixel_data[ref_idx + ch] as i16)
                                    .abs() as u8,
                            );
                        }
                    }

                    if local_max > threshold {
                        needs_split = true;
                        break;
                    } else {
                        needs_split = false;
                    }
                }

                if needs_split {
                    let mut offset_correction = 0;
                    for offset in 1..(aura_margin * 2) {
                        let mut local_max = 0;

                        for x in 0..img_width {
                            let ref_idx = ((split_line - offset) * img_width + x) * num_channels;
                            for ch in 0..num_channels {
                                local_max = local_max.max(
                                    (pixel_data[compare_base_idx + ch] as i16
                                        - pixel_data[ref_idx + ch] as i16)
                                        .abs() as u8,
                                );
                            }
                        }

                        offset_correction = offset / 2;
                        if local_max > threshold {
                            break;
                        }
                    }

                    split_line -= offset_correction;
                }
            }

            crop_boundaries.push(split_line);
            next_crop_start = split_line + crop_height;
        }

        for x in 0..img_width {
            let base_current = (row_start_idx + x) * num_channels;
            let base_next = (next_row_start_idx + x) * num_channels;

            for ch in 0..num_channels {
                max_diff = max_diff.max(
                    (pixel_data[base_current + ch] as i16 - pixel_data[base_next + ch] as i16).abs()
                        as u8,
                );
            }
        }

        if max_diff <= threshold {
            split_line = row;
        }
    }

    crop_boundaries.push(img_height);

    for i in 0..crop_boundaries.len() - 1 {
        let start_y = crop_boundaries[i];
        let end_y = crop_boundaries[i + 1];
        let height = end_y - start_y;

        let cropped_slice =
            &pixel_data[(start_y * img_width * num_channels)..(end_y * img_width * num_channels)];
        let shape = Shape::new(height, img_width, Some(num_channels));
        let img_data = ImgData::U8(cropped_slice.to_owned());

        let _ = svec_save(
            SVec::new(shape, img_data),
            &out_path.join(format!("{name}_{i}.png")),
        );
    }
}
