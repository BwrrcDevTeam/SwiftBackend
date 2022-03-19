use ndarray::{Array, Array2, ArrayView2, ArrayView3, Dim, s};
use ndarray;
use nshare::ToNdarray3;
use onnxruntime::environment::Environment;
use onnxruntime::{GraphOptimizationLevel, LoggingLevel};
use onnxruntime::session::Session;
use onnxruntime::tensor::OrtOwnedTensor;

use serde::{Deserialize, Serialize};

// 注意: 这个快速排序的方向是从大到小
fn quick_sort_helper(array: &mut Vec<(usize, usize, f32)>, left: usize, right: usize) {
    // 不处理k右边部分的数据
    if left < right {
        let (mut i, mut j, x) = (left, right, array[left]);
        while i < j {
            while i < j && array[j].2 < x.2 {
                j -= 1;
            }
            if i < j {
                array[i] = array[j];
                i += 1;
            }
            while i < j && array[i].2 >= x.2 {
                i += 1;
            }
            if i < j {
                array[j] = array[i];
                j -= 1;
            }
        }
        array[i] = x;
        if i > 0 {
            quick_sort_helper(array, left, i - 1);
        }
        quick_sort_helper(array, i + 1, right);
    }
}


// 利用快速排序把带坐标信息的数组排序
fn quick_sort(origin: Vec<(usize, usize, f32)>) -> Vec<(usize, usize, f32)> {
    let mut array = origin;
    let left = 0;
    let right = array.len() - 1;
    quick_sort_helper(&mut array, left, right);
    array
}

// 从一个Heatmap中获取前K大的点的坐标以及其对应的热度值
pub fn top_k(heatmap: &Array2<f32>, k: usize) -> Vec<(usize, usize, f32)> {
    quick_sort(heatmap.indexed_iter().map(|((x, y), v)| { (x, y, *v) }).collect()).iter().take(k).cloned().collect()
}

// padding操作
fn pad(array: &Array2<f32>, padding: usize) -> Array2<f32> {
    let (height, width) = array.dim();
    let mut padded_array = Array::zeros((height + padding * 2, width + padding * 2));
    padded_array.slice_mut(s![padding..height + padding, padding..width + padding])
        .assign(&array);
    padded_array
}

// 最大池化
// 于2.11除虫完毕
fn max_pooling(array: &Array2<f32>, kernel: usize, stride: usize, padding: usize) -> Array2<f32> {
    let (height, width) = array.dim();
    let out_height: usize = (height - kernel + 2 * padding) / stride + 1;
    let out_width: usize = (width - kernel + 2 * stride) / stride + 1;
    // 对原始数据进行padding
    let array = pad(array, padding);
    let mut out = Array2::zeros((out_height, out_width));

    for i in 0..out_height {
        for j in 0..out_width {
            let mut max = array[[i * stride, j * stride]];
            for k in 0..kernel {
                for l in 0..kernel {
                    let value = array[[i * stride + k, j * stride + l]];
                    if value > max {
                        max = value;
                    }
                }
            }
            out[[i, j]] = max;
        }
    }
    out
}


pub fn heatmap_nms(hm: &Array2<f32>, kernel: usize) -> Array2<f32> {
    // 对其执行最大值池化
    let padding = (kernel - 1) / 2;
    let hm_max = max_pooling(&hm, kernel, 1, padding);
    // 保留值与最大值相等的点
    let mut hm = (*hm).clone();
    hm.indexed_iter_mut()
        .map(|((y, x), score)| {
            if score != &hm_max[[y, x]] {
                *score *= 0.0
            }
        })
        .for_each(drop);
    hm
}


pub fn make_env() -> Result<Environment, onnxruntime::error::OrtError> {
    let environment = Environment::builder()
        .with_name("SwiftDet")
        .with_log_level(LoggingLevel::Error)
        .build()?;
    Ok(environment)
}


fn make_session<'a>(env: &'a Environment, config: &'a DetectConfig) -> Session<'a> {
    let num = num_cpus::get();
    let session = env
        .new_session_builder().expect("无法创建会话")
        .with_optimization_level(GraphOptimizationLevel::All).unwrap()
        .with_number_threads(num as i16).unwrap()
        .with_model_from_file(&config.model_path).unwrap();
    session
}

#[derive(Deserialize, Debug, Serialize, Clone)]
pub struct BBox {
    pub x_min: i32,
    pub y_min: i32,
    pub x_max: i32,
    pub y_max: i32,
    pub score: f32,
}


fn preprocess(img: &ndarray::Array3<u8>, config: &DetectConfig) -> ndarray::Array3<f32> {
    let mut img = img.mapv(|x| x as f32 / 255.0);
    // 先分离通道
    let mut r = img.slice_mut(s![0..1, .., ..]);
    r -= config.mean[0];
    r /= config.std[0];
    let mut g = img.slice_mut(s![1..2, .., ..]);
    g -= config.mean[1];
    g /= config.std[1];
    let mut b = img.slice_mut(s![2..3, .., ..]);
    b -= config.mean[2];
    b /= config.std[2];
    img
}

pub struct DetectConfig {
    pub mean: [f32; 3],
    pub std: [f32; 3],
    pub window_size: (usize, usize),
    // 宽 高
    pub overlap: u8,
    pub tile_max_num: u16,
    pub input_size: (usize, usize),
    // 宽 高
    pub batch_size: u8,
    pub heatmap_size: (usize, usize),
    // 宽 高
    pub model_path: String,
}

fn resize_img(img: ndarray::Array3<f32>, width_scale: f32, height_scale: f32) -> ndarray::Array3<f32> {
    if (width_scale == 1.0) && (height_scale == 1.0) {
        return img.clone();
    }
    let (_, height, width) = img.dim();
    let new_width = (width as f32 * width_scale).round() as usize;
    let new_height = (height as f32 * height_scale).round() as usize;
    let mut new_img = ndarray::Array3::<f32>::zeros((3, new_height, new_width));
    for x in 0..new_width {
        for y in 0..new_height {
            let x_src = (x as f32 / width_scale) as usize;
            let y_src = (y as f32 / height_scale) as usize;
            new_img[[0, y, x]] = img[[0, y_src, x_src]];
            new_img[[1, y, x]] = img[[1, y_src, x_src]];
            new_img[[2, y, x]] = img[[2, y_src, x_src]];
        }
    }
    new_img
}


fn tile_edge(length: usize, window_length: usize) -> (u8, u16) {
    let num = length / window_length;
    let avg_length = length / num;
    (num as u8, avg_length as u16)
}

struct Metadata {
    start_x: usize,
    start_y: usize,
    width_scale: f32,
    height_scale: f32,
}

// 拆分区块
fn split_tiles(img: ndarray::Array3<f32>, config: &DetectConfig) -> Vec<(ndarray::Array4<f32>, Metadata)> {
    let (_, origin_height, origin_width) = img.dim();
    let (window_width, window_height) = config.window_size;
    let (input_width, input_height) = config.input_size;
    // 进行全局缩放 保证图片宽高大于两倍的window_size
    let mut height_scale = 1.0;
    let mut width_scale = 1.0;
    if origin_height < (window_height * 2) {
        height_scale = (window_height * 2) as f32 / origin_height as f32;
    }
    // 对宽度进行缩放
    if origin_width < (window_width * 2) {
        width_scale = (window_width * 2) as f32 / origin_width as f32;
    }
    // 执行变换
    let img = resize_img(img, width_scale, height_scale);
    let (_, current_height, current_width) = img.dim();
    // 步骤二：分割区块
    let (width_num, width_tile) = tile_edge(current_width, window_width);
    let (height_num, height_tile) = tile_edge(current_height, window_height);
    // 开始裁剪
    let mut tiles = Vec::new();
    let overlap = (config.overlap / 2) as usize;

    let batch_shape = Dim((
        config.batch_size as usize,
        3 as usize,
        config.input_size.0,
        config.input_size.1
    ));

    // 从左到右 从上到下
    for yi in 0..height_num {
        for xi in 0..width_num {
            let mut start_y = yi as usize * height_tile as usize;
            let mut end_y = start_y + height_tile as usize;
            let mut start_x = xi as usize * width_tile as usize;
            let mut end_x = start_x + width_tile as usize;

            // 如果不是贴边的区块，则为其添加overlap
            let tile_width_scale = if start_x != 0 && (end_x + overlap <= current_width) {
                start_x -= overlap;
                end_x += overlap;
                (width_tile + config.overlap as u16) as f32 / input_width as f32
            } else {
                width_tile as f32 / input_width as f32
            };

            // 对高度进行类似操作
            let tile_height_scale = if start_y != 0 && (usize::from(end_y + overlap) <= current_height) {
                start_y -= overlap;
                end_y += overlap;
                (height_tile + config.overlap as u16) as f32 / input_height as f32
            } else {
                height_tile as f32 / input_height as f32
            };

            // 选取区块
            let slice = s![.., start_y..end_y, start_x..end_x];

            let tile = img.slice(slice);
            let tile = tile.to_owned();
            // 对区块进行缩放
            let tile = resize_img(tile, 1. / tile_width_scale, 1. / tile_height_scale);
            let tile_shape = (*tile.shape()).to_vec();

            let tile = tile.into_shape(batch_shape.clone());
            if tile.is_err() {
                eprintln!("在裁剪区块时出错: {:?} {:?}", tile.err(), tile_shape);
                eprintln!("{}-{} x {}-{}", start_x, end_x, start_y, end_y);
                eprintln!("ws:{} hs:{}", tile_width_scale, tile_height_scale);
            } else {
                let tile = tile.unwrap();
                tiles.push((
                    tile,
                    Metadata {
                        start_x,
                        start_y,
                        width_scale: tile_width_scale / width_scale,
                        height_scale: tile_height_scale / height_scale,
                    }
                ));
            }
        }
    }
    tiles
}


// 验证完毕
fn sigmoid(x: f32) -> f32 {
    1. / (1. + (-x).exp())
}

// 从神经网络输出的Heatmap和WH中提取检测框
fn decode_heatmap(hm: &ArrayView2<f32>, wh: &ArrayView3<f32>, config: &DetectConfig) -> Vec<BBox> {
    // 对Heatmap执行NMS
    let hm = hm.mapv(sigmoid);
    let hm = heatmap_nms(&hm, 3);
    let mut peaks = top_k(&hm, config.tile_max_num as usize);
    let mut boxes = Vec::new();
    let width_scale = config.input_size.0 as f32 / config.heatmap_size.0 as f32;
    let height_scale = config.input_size.1 as f32 / config.heatmap_size.1 as f32;

    for peak in peaks.iter_mut() {
        let width = wh.get((peak.0, peak.1, 0)).unwrap();
        let height = wh.get((peak.0, peak.1, 0)).unwrap();
        if width >= &100. {
            eprintln!("警告: 不合理的宽度: {}", width);
        }
        if height >= &100. {
            eprintln!("警告: 不合理的高度: {}", height);
        }
        boxes.push(BBox {
            x_min: ((peak.0 as f32 - width / 2.) * width_scale) as i32,
            x_max: ((peak.0 as f32 + width / 2.) * width_scale) as i32,
            y_min: ((peak.1 as f32 - height / 2.) * height_scale) as i32,
            y_max: ((peak.1 as f32 + height / 2.) * height_scale) as i32,
            score: peak.2,
        })
    }
    boxes
}

// 已检查
fn apply_metadata(mut boxes: Vec<BBox>, metadata: &Metadata) -> Vec<BBox> {
    // 根据tile的元数据对检测框进行缩放
    for box_ in boxes.iter_mut() {
        box_.x_min = (box_.x_min as f32 * metadata.width_scale) as i32 + metadata.start_x as i32;
        box_.x_max = (box_.x_max as f32 * metadata.width_scale) as i32 + metadata.start_x as i32;
        box_.y_min = (box_.y_min as f32 * metadata.height_scale) as i32 + metadata.start_y as i32;
        box_.y_max = (box_.y_max as f32 * metadata.height_scale) as i32 + metadata.start_y as i32;
    }
    boxes
}

fn iou(a: &BBox, b: &BBox) -> f32 {
    let x_min = core::cmp::max(a.x_min, b.x_min);
    let x_max = core::cmp::min(a.x_max, b.x_max);
    let y_min = core::cmp::max(a.y_min, b.y_min);
    let y_max = core::cmp::min(a.y_max, b.x_max);
    let overlap = (x_max - x_min + 1) * (y_max - y_min + 1);
    if overlap <= 0 {
        return 0.;
    }
    let area_a = (a.x_max - a.x_min + 1) * (a.y_max - a.y_min + 1);
    let area_b = (b.x_max - b.x_min + 1) * (b.y_max - b.y_min + 1);
    // Area of overlap / Area of Union
    return overlap as f32 / (area_a + area_b - overlap) as f32;
}

fn sort_boxes(boxes: &mut Vec<BBox>) {
    // 用快速排序对检测框进行排序
    // score大的排在后面
    boxes.sort_unstable_by(|a, b| a.score.partial_cmp(&b.score).unwrap());
}

fn soft_nms_method(x: f32) -> f32 {
    // 高斯加权
    let sigma = 0.5;
    (-((x * x) / sigma)).exp()
}

fn soft_nms(mut boxes: Vec<BBox>) -> Vec<BBox> {
    // 执行soft NMS算法
    // 这里去掉了排序检测框的部分 默认输入的检测框是按照score排序的
    let mut keep = Vec::new();

    // 重复执行直到没有检测框可以保留
    while !boxes.is_empty() {
        // 排序
        sort_boxes(&mut boxes);
        // M是分数最高的检测框(最后一个)
        let m = boxes.pop().unwrap();
        // 从boxes中删除 与M的IOU大于thresh的检测框
        for b in boxes.iter_mut() {
            // 计算交集 越大(->1)说明越接近
            let iou = iou(&m, &b);
            // 修改b的score
            b.score = b.score * soft_nms_method(iou);
        }
        // 最后再将m放到keep里
        keep.push(m);
    }
    keep
}



// 执行检测
pub fn detect<F: FnMut(&usize, &usize)>(image_path: &str, config: DetectConfig, env: Environment, mut progress_callback: F, do_nms: bool) -> Result<Vec<BBox>, String>
{
    match image::open(image_path) {
        Ok(image) => {
            let image = image.into_rgb8();
            let image = image.into_ndarray3();

            let array_image = preprocess(&image, &config);
            let batches = split_tiles(array_image, &config);
            let mut sess = make_session(&env, &config);
            let mut current = 1;
            let total = batches.len();
            let mut all_boxes = Vec::new();

            for (batch, metadata) in batches {
                // 调用Python的回调函数
                progress_callback(&current, &total);

                let outputs: Vec<OrtOwnedTensor<f32, _>> = sess.run(vec![batch]).unwrap();
                let hm = outputs[0].t();
                let wh = outputs[1].t();
                // let hm = Array::from(hm.iter().copied().collect::<Vec<f32>>());
                let hm = hm.into_shape(config.heatmap_size)
                    .unwrap();
                let wh = wh.into_shape((config.heatmap_size.1, config.heatmap_size.0, 2 as usize))
                    .unwrap();
                let tile_boxes = apply_metadata(decode_heatmap(&hm, &wh, &config), &metadata);

                all_boxes.extend(tile_boxes);
                current += 1;
            }
            if do_nms {
                Ok(soft_nms(all_boxes))
            } else {
                Ok(all_boxes)
            }
        }
        Err(e) => {
            Err(format!("无法打开图像:{}", e))
        }
    }
}
