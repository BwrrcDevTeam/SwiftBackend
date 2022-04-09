use wither::Model;
use serde::{Deserialize, Serialize};
use serde_json::json;
use wither::bson::DateTime;
use wither::bson::oid::ObjectId;
use crate::models::SearchById;

#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "records")]
pub struct Record {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub num: i16,
    pub position: String,
    pub time: DateTime,
    pub collaborators: Vec<String>,
    pub description: String,
    pub weather: String,
    // 属于的小组
    pub group: String,
    // 创建这个记录的用户
    pub user: String,
    // 这个记录属于的项目
    pub project: String,
    // 附件
    pub attachments: Vec<String>,
    // 更多可选项
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
    // 是否被推荐
    pub is_recommended: Option<bool>,
}

impl SearchById for Record {}

impl Record {
    pub fn to_response(self) -> serde_json::Value {
        json!({
            "id": self.id.unwrap().to_string(),
            "num": self.num,
            "position": self.position,
            "time": self.time.timestamp(),
            "collaborators": self.collaborators,
            "description": self.description,
            "group": self.group,
            "user": self.user,
            "project": self.project,
            "weather": self.weather,
            "attachments": self.attachments,
            "num_of_nests": self.num_of_nests,
            "return_time": self.return_time,
            "return_direction": self.return_direction,
            "nest_height": self.nest_height,
            "nest_area": self.nest_area,
            "nest_material": self.nest_material,
        })
    }
}