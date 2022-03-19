use wither::bson::{DateTime, doc};
use wither::bson::oid::ObjectId;
use swift_det_lib::{BBox, DetectConfig};
use crate::models::storage::Storage;
use wither::Model;
use serde::{Serialize, Deserialize};
use wither::mongodb::Database;
use crate::models::SearchById;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "detections")]
pub struct Detection {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub creator: String,
    pub created_at: DateTime,
    pub status: String,
    pub attachment: String,
    pub window_size: isize,
    pub overlap: i8,
    pub tile_max_num: i16,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Vec<BBox>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
}


const MEAN: [f32; 3] = [1.785167, 1.533696, 1.380282];
const STD: [f32; 3] = [1.667162, 1.44502, 1.320071];
const INPUT_SIZE: (usize, usize) = (800, 800);
const HEATMAP_SIZE: (usize, usize) = (200, 200);

impl Detection {
    pub fn get_config(&self, model_path: String) -> DetectConfig {
        DetectConfig {
            window_size: (self.window_size as usize, self.window_size as usize),
            overlap: self.overlap as u8,
            tile_max_num: self.tile_max_num as u16,
            input_size: INPUT_SIZE,
            batch_size: 1,
            heatmap_size: HEATMAP_SIZE,
            model_path,
            mean: MEAN,
            std: STD,
        }
    }
    pub async fn get_attachment(&self, db: &Database) -> Option<Storage> {
        Storage::by_id(db, &self.attachment).await
    }
    pub async fn to_status(&self) -> DetectionStatusResponse {
        DetectionStatusResponse {
            status: self.status.clone(),
            current: self.current.clone(),
            total: self.total.clone(),
        }
    }
    pub async fn to_info(&self) -> DetectionInfoResponse {
        DetectionInfoResponse {
            id: self.id.clone().unwrap().to_hex(),
            creator: self.creator.clone(),
            created_at: self.created_at.clone(),
            status: self.status.clone(),
            attachment: self.attachment.clone(),
            window_size: self.window_size,
            overlap: self.overlap,
            tile_max_num: self.tile_max_num,
            model_name: self.model_name.clone(),
            current: self.current.clone(),
            total: self.total.clone(),
            threshold: self.threshold.clone(),
        }
    }
}

// 只包含状态信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DetectionStatusResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<isize>,
}

// 包含除了Result之外的信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DetectionInfoResponse {
    pub id: String,
    pub creator: String,
    pub created_at: DateTime,
    pub attachment: String,
    pub status: String,
    pub window_size: isize,
    pub overlap: i8,
    pub tile_max_num: i16,
    pub model_name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total: Option<isize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub threshold: Option<f64>,
}

impl SearchById for Detection {}