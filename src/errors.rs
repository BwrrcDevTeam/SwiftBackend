use std::fmt::Display;
use log::error;
use serde::de::StdError;
use tide::{Response, StatusCode};
use tide::prelude::*;

#[derive(Debug, Clone)]
pub enum AppErrors {
    CrossPermissionError(i8, Vec<i8>),
    // 越权访问
    // BadRequest,
    // // 请求错误
    ValidationError(serde_json::Value),
    // 验证错误
}


impl Display for AppErrors {
    fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Ok(())
    }
}

impl StdError for AppErrors {}


pub async fn handle(mut res: Response) -> tide::Result<Response> {
    // IO错误 可能在读取文件 检测图片的时候触发
    if let Some(err) = res.downcast_error::<async_std::io::Error>() {
        let err = err.to_string();
        res.set_status(StatusCode::InternalServerError);

        res.set_body(json!({
            "code": 500,
            "message": {
                "cn": "服务器IO错误",
                "en": "Server IO error"
            },
            "description": err
        }));
        res.set_content_type("application/json");
    }


    if let Some(err) = res.downcast_error::<AppErrors>() {
        // err现在是我的了 :-)

        let err = err.to_owned();
        match err {
            // 越权 check perm的时候触发
            AppErrors::CrossPermissionError(current, allowed) => {
                res.set_body(json!({
                    "code": 1,
                    "message": {
                        "cn": "越权访问",
                        "en": "Cross permission"
                    },
                    "description": {
                        "current": current,
                        "allowed": allowed
                    }
                }));
                res.set_status(StatusCode::Forbidden);
                res.set_content_type("application/json");
            }
            // // parsing form的时候触发
            // AppErrors::BadRequest => {
            //     res.set_body(json!({
            //         "code": 4,
            //         "message": {
            //             "cn": "服务器无法理解请求的内容, 请检查客户端版本",
            //             "en": "Server cannot understand the request, please check the client version"
            //         },
            //         "description": "AppErrors::BadRequest"
            //     }));
            //     res.set_status(StatusCode::BadRequest);
            //     res.set_content_type("application/json");
            // }
            // 验证错误
            AppErrors::ValidationError(json_response) => {
                res.set_body(json_response);

                res.set_status(StatusCode::BadRequest);

                res.set_content_type("application/json");
            }
        }
    }
    // 数据库和事物错误
    if let Some(err) = res.downcast_error::<wither::WitherError>() {
        // let err = err.to_owned();
        error!("Wither Error: {:?}", err);
        res.set_body(json!({
            "code": 500,
            "message": {
                "cn": "服务器数据库错误",
                "en": "Server database error"
            },
            // "description": err.to_string()
        }));
        res.set_status(StatusCode::InternalServerError);
        res.set_content_type("application/json");
    }

    if res.status() == StatusCode::NotFound {
        let body = res.take_body();
        if body.is_empty().unwrap_or(true) {
            res.set_body(json!({
                "code": 404,
                "message": {
                    "cn": "此终端不存在",
                    "en": "This endpoint does not exist"
                },
                "description": "NotFound"
            }));
            res.set_content_type("application/json");
        } else {
            res.set_body(body);
        }
    }

    if res.status() == StatusCode::MethodNotAllowed {
        let body = res.take_body();
        if body.is_empty().unwrap_or(true) {
            res.set_body(json!({
                "code": 405,
                "message": {
                    "cn": "此终端不存在",
                    "en": "This endpoint does not exist"
                },
                "description": "MethodNotAllowed"
            }));
            res.set_content_type("application/json");
        } else {
            res.set_body(body);
        }
    }

    if res.status() == StatusCode::Unauthorized {
        let body = res.take_body();
        if body.is_empty().unwrap_or(true) {
            res.set_body(json!({
                "code": 401,
                "message": {
                    "cn": "请求未授权",
                    "en": "Request unauthorized"
                },
                "description": "Unauthorized"
            }));
            res.set_content_type("application/json");
        } else {
            res.set_body(body);
        }
    }
    if res.status() == StatusCode::UnprocessableEntity {
        let body = res.take_body();
        if body.is_empty().unwrap_or(true) {
            res.set_body(json!({
                "code": 422,
                "message": {
                    "cn": "无法处理请求",
                    "en": "Unable to process the request"
                },
                "description": "UnprocessableEntity"
            }));
            res.set_content_type("application/json");
        } else {
            res.set_body(body);
        }
    }
    Ok(res)
}