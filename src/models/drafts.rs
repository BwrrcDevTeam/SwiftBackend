use wither::bson::oid::ObjectId;
use wither::Model;
use serde::{Serialize, Deserialize};
use serde_json::json;
use wither::bson::doc;
use wither::mongodb::Database;

// 草稿
#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "drafts")]
pub struct RecordDraft {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    // 绝大部分都是Option
    pub position: Option<String>,
    // 选择的填报点
    pub collaborators: Option<Vec<String>>,
    // 协作
    pub num: Option<i32>,
    // 最重要的 雨燕数量
    pub time: Option<i32>,
    // 时间戳
    pub description: Option<String>,
    // 描述
    pub user: String,
    // 创建者
    // 鸟巢数量
    pub num_of_nests: Option<i16>,
    // 回巢时间
    pub return_time: Option<String>,
    // 回巢方向
    pub return_direction: Option<String>,
    // 巢区高度
    pub nest_height: Option<f64>,
    // 巢区面积
    pub nest_area: Option<f64>,
    // 巢材
    pub nest_material: Option<String>,
    pub weather: Option<String>,
}


#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "drafts")]
pub struct DetectionDraft {
    // todo: 在未来实现
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
}

impl RecordDraft {
    pub async fn by_user(db: &Database, user: &String) -> Option<Self> {
        if let Ok(result) = RecordDraft::find_one(db, doc! {"user": user}, None).await {
            result
        } else {
            None
        }
    }
    pub fn to_response(self) -> serde_json::Value {
        json!( {
            "position": self.position,
            "collaborators": self.collaborators,
            "num": self.num,
            "time": self.time,
            "description": self.description,
            "num_of_nests": self.num_of_nests,
            "return_time": self.return_time,
            "return_direction": self.return_direction,
            "nest_height": self.nest_height,
            "nest_area": self.nest_area,
            "nest_material": self.nest_material,
            "weather": self.weather,
        })
    }
}