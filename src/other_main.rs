use std::{
    any::Any,
    ops::Deref,
    panic::catch_unwind,
    sync::{Arc, Mutex},
};

use actix_web::{HttpResponse, Responder, get, web::Query};
use serde::Deserialize;
use simple_search_engine_lib::searchable::Searchable;

use crate::{
    DATA_MANAGER,
    file_handling::{document_type_ids::VIDEO_DATA_TYPE_ID, video_data::VideoData},
};

#[derive(Deserialize, Debug)]
struct SearchQuery {
    query: String,
}

#[get("/search")]
pub async fn search_endpoint(params: Query<SearchQuery>) -> impl Responder {
    let error_response = HttpResponse::BadRequest().finish();
    let query_str = params.query.as_str();
    if query_str.is_empty() {
        return error_response;
    }

    let mut result: String = "{\"result\": [".to_string();
    let search_results: Vec<Arc<Mutex<dyn Searchable>>> =
        match catch_unwind(|| -> Vec<Arc<Mutex<dyn Searchable>>> {
            let data_manager_mutg = DATA_MANAGER.lock().unwrap();
            let data_manager = data_manager_mutg.deref();
            // data_manager.search_video_by_name(query_str, 150)
            data_manager.search(query_str, 150)
        }) {
            Ok(res) => res,
            Err(_) => return error_response,
        };
    {
        let mut first: bool = true;
        for search_result_arc in search_results {
            if !first {
                result.push_str(", ");
            } else {
                first = false;
            }
            let search_result_mutg = search_result_arc.lock().unwrap();
            let search_result = search_result_mutg.deref();
            let doc_type: u64 = search_result.get_document_type();
            // let doc_type: u64 = {
            //     let search_result_mutg = search_result_arc.lock().unwrap();
            //     let search_result = search_result_mutg.deref();
            //     search_result.get_document_type()
            // };
            if doc_type == VIDEO_DATA_TYPE_ID {
                let search_result_any = search_result as &dyn Any;
                let search_result_casted: &VideoData =
                    search_result_any.downcast_ref::<VideoData>().unwrap();
                result.push_str(
                    format!(
                        "{{\"doc_type\": {doc_type}, \"id\": {}}}",
                        search_result_casted.get_id()
                    )
                    .as_str(),
                );
            } else {
                unimplemented!("Invalid Searchable document type id: {}", doc_type);
            }
        }
    }

    result.push_str("]}");
    HttpResponse::Ok().body(result)
}
