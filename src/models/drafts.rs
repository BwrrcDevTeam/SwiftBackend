use wither::bson::oid::ObjectId;
use wither::Model;
use serde::{Serialize, Deserialize};

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
    pub description: Option<String>, // 描述
}


#[derive(Debug, Model, Serialize, Deserialize, Clone)]
#[model(collection_name = "drafts")]
pub struct DetectionDraft {
    // todo: 在未来实现
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
}