use futures::{future, Future};
use serde_json::Value;
use errors::ClientError;
use protocol;
use serde_json::{from_value, to_value, Map};
use serde::Serialize;
use structs::{ModifySelection, ViewId};

/// A future returned by all the `Client`'s method.
pub type ClientResult<T> = Box<Future<Item = T, Error = ClientError> + Send>;

/// A client to send notifications and request to xi-core.
#[derive(Clone)]
pub struct Client(pub protocol::Client);

fn get_edit_params<T: Serialize>(
    view_id: ViewId,
    method: &str,
    params: Option<T>,
) -> Result<Value, ClientError> {
    let params_value = if let Some(params) = params {
        to_value(params)?
    } else {
        json!([])
    };

    Ok(json!({
        "method": method,
        "view_id": view_id,
        "params": params_value,
    }))
}


impl Client {
    /// Send a notification to the core. Most (if not all) notifications
    /// supported by the core are already implemented, so this method
    /// should not be necessary in most cases.
    pub fn notify(&mut self, method: &str, params: Value) -> ClientResult<()> {
        info!(">>> notification: method={}, params={}", method, &params);
        Box::new(
            self.0
                .notify(method, params)
                .map_err(|_| ClientError::NotifyFailed),
        )
    }

    /// Send a request to the core. Most (if not all) notifications
    /// supported by the core are already implemented, so this method
    /// should not be necessary in most cases.
    pub fn request(&mut self, method: &str, params: Value) -> ClientResult<Value> {
        info!(">>> request : method={}, params={}", method, &params);
        Box::new(self.0.request(method, params).then(
            |response| match response {
                Ok(Ok(value)) => Ok(value),
                Ok(Err(value)) => Err(ClientError::ErrorReturned(value)),
                Err(_) => Err(ClientError::RequestFailed),
            },
        ))
    }

    pub fn edit_request<T: Serialize>(
        &mut self,
        view_id: ViewId,
        method: &str,
        params: Option<T>)-> ClientResult<Value> {
            match get_edit_params(view_id, method, params) {
                Ok(value) => self.request("edit", value),
                Err(e) => Box::new(future::err(e)),
        }
    }

    /// Send an "edit" notification. Most (if not all) "edit" commands are
    /// already implemented, so this method should not be necessary in most
    /// cases.
    pub fn edit_notify<T: Serialize>(
        &mut self,
        view_id: ViewId,
        method: &str,
        params: Option<T>,
    ) -> ClientResult<()> {
        match get_edit_params(view_id, method, params) {
            Ok(value) => self.notify("edit", value),
            Err(e) => Box::new(future::err(e)),
        }
    }

    /// Send an "scroll" notification
    /// ```ignore
    /// {"method":"edit","params":{"method":"scroll","params":[21,80],
    /// "view_id":"view-id-1"}}
    /// ```
    pub fn scroll(&mut self, view_id: ViewId, first_line: u64, last_line: u64) -> ClientResult<()> {
        self.edit_notify(view_id, "scroll", Some(json!([first_line, last_line])))
    }

    pub fn goto_line(&mut self, view_id: ViewId, line: u64) -> ClientResult<()> {
        self.edit_notify(view_id, "goto_line", Some(json!({"line": line})))
    }

    pub fn copy(&mut self, view_id: ViewId) -> ClientResult<Value> {
        self.edit_request(view_id, "copy", None as Option<Value>)
    }

    pub fn paste(&mut self, view_id: ViewId, buffer: &str) -> ClientResult<()> {
        self.edit_notify(view_id, "paste", Some(json!({"chars": buffer})))
    }

    pub fn cut(&mut self, view_id: ViewId) -> ClientResult<Value> {
        self.edit_request(view_id, "cut", None as Option<Value>)
    }

    pub fn undo(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "undo", None as Option<Value>)
    }

    pub fn redo(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "redo", None as Option<Value>)
    }

    pub fn find(
        &mut self,
        view_id: ViewId,
        search_term: &str,
        case_sensitive: bool,
        regex: bool,
        whole_words: bool,
    ) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "find",
            Some(json!({
                "chars": search_term,
                "case_sensitive": case_sensitive,
                "regex": regex,
                "whole_words": whole_words}))
        )
    }

    fn find_other(
        &mut self,
        view_id: ViewId,
        command: &str,
        wrap_around: bool,
        allow_same: bool,
        modify_selection: ModifySelection,
    ) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            command,
            Some(json!({
                "wrap_around": wrap_around,
                "allow_same": allow_same,
                "modify_selection": modify_selection}))
        )
    }

    pub fn find_next(
        &mut self,
        view_id: ViewId,
        wrap_around: bool,
        allow_same: bool,
        modify_selection: ModifySelection,
    ) -> ClientResult<()> {
        self.find_other(
            view_id,
            "find_next",
            wrap_around,
            allow_same,
            modify_selection)
    }

    pub fn find_prev(
        &mut self,
        view_id: ViewId,
        wrap_around: bool,
        allow_same: bool,
        modify_selection: ModifySelection,
    ) -> ClientResult<()> {
        self.find_other(
            view_id,
            "find_previous",
            wrap_around,
            allow_same,
            modify_selection)
    }

    pub fn find_all(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "find_all", None as Option<Value>)
    }

    pub fn highlight_find(&mut self, view_id: ViewId, visible: bool) -> ClientResult<()> {
        self.edit_notify(view_id, "highlight_find", Some(json!({"visible": visible})))
    }

    pub fn left(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "move_left", None as Option<Value>)
    }

    pub fn left_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "move_left_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn right(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "move_right", None as Option<Value>)
    }

    pub fn right_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "move_right_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn up(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "move_up", None as Option<Value>)
    }

    pub fn up_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "move_up_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn down(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "move_down", None as Option<Value>)
    }

    pub fn down_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "move_down_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn backspace(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.del(view_id)
    }

    pub fn delete(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "delete_forward", None as Option<Value>)
    }

    pub fn del(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "delete_backward", None as Option<Value>)
    }

    pub fn page_up(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "scroll_page_up", None as Option<Value>)
    }

    pub fn page_up_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "page_up_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn page_down(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "scroll_page_down", None as Option<Value>)
    }

    pub fn page_down_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "page_down_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn line_start(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "move_to_left_end_of_line", None as Option<Value>)
    }

    pub fn line_start_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "move_to_left_end_of_line_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn line_end(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "move_to_right_end_of_line", None as Option<Value>)
    }

    pub fn line_end_sel(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(
            view_id,
            "move_to_right_end_of_line_and_modify_selection",
            None as Option<Value>,
        )
    }

    pub fn select_all(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "select_all", None as Option<Value>)
    }

    pub fn collapse_selections(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "collapse_selections", None as Option<Value>)
    }

    pub fn insert_newline(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "insert_newline", None as Option<Value>)
    }

    pub fn insert_tab(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "insert_tab", None as Option<Value>)
    }

    pub fn f1(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "debug_rewrap", None as Option<Value>)
    }

    pub fn f2(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.edit_notify(view_id, "debug_test_fg_spans", None as Option<Value>)
    }

    pub fn char(&mut self, view_id: ViewId, ch: char) -> ClientResult<()> {
        self.edit_notify(view_id, "insert", Some(json!({ "chars": ch })))
    }

    // FIXME: handle modifier and click count
    pub fn click(&mut self, view_id: ViewId, line: u64, column: u64) -> ClientResult<()> {
        self.edit_notify(view_id, "click", Some(json!([line, column, 0, 1])))
    }

    pub fn click_point_select(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "point_select";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn click_toggle_sel(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "toggle_sel";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn click_range_select(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "range_select";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn click_line_select(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "range_select";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn click_word_select(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "word_select";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn click_multi_line_select(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "multi_line_select";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn click_multi_word_select(
        &mut self,
        view_id: ViewId,
        line: u64,
        column: u64,
    ) -> ClientResult<()> {
        let ty = "multi_word_select";
        self.edit_notify(
            view_id,
            "gesture",
            Some(json!({"line": line, "col": column, "ty": ty,})),
        )
    }

    pub fn drag(&mut self, view_id: ViewId, line: u64, column: u64) -> ClientResult<()> {
        self.edit_notify(view_id, "drag", Some(json!([line, column, 0])))
    }

    /// send a `"new_view"` request to the core.
    /// ```ignore
    /// {"id":1,"method":"new_view","params":{"file_path":"foo/test.txt"}}
    /// ```
    pub fn new_view(&mut self, file_path: Option<String>) -> ClientResult<ViewId> {
        let params = if let Some(file_path) = file_path {
            json!({ "file_path": file_path })
        } else {
            json!({})
        };
        let result = self.request("new_view", params)
            .and_then(|result| from_value::<ViewId>(result).map_err(From::from));
        Box::new(result)
    }

    /// send a `"close_view"` notifycation to the core.
    pub fn close_view(&mut self, view_id: ViewId) -> ClientResult<()> {
        self.notify("close_view", json!({ "view_id": view_id }))
    }

    pub fn save(&mut self, view_id: ViewId, file_path: &str) -> ClientResult<()> {
        let params = json!({"view_id": view_id, "file_path": file_path});
        Box::new(self.notify("save", params).and_then(|_| Ok(())))
    }

    pub fn set_theme(&mut self, theme: &str) -> ClientResult<()> {
        let params = json!({ "theme_name": theme });
        Box::new(self.notify("set_theme", params).and_then(|_| Ok(())))
    }

    pub fn client_started(&mut self, config_dir: Option<&str>, client_extra_dir: Option<&str>) -> ClientResult<()> {
        let mut params = Map::new();
        if let Some(path) = config_dir {
            let _ = params.insert("config_dir".into(), json!(path));
        }
        if let Some(path) = client_extra_dir {
            let _ = params.insert("client_extra_dir".into(), json!(path));
        }
        self.notify("client_started", params.into())
    }

    pub fn start_plugin(&mut self, view_id: ViewId, name: &str) -> ClientResult<()> {
        let params = json!({"view_id": view_id, "plugin_name": name});
        Box::new(self.notify("start", params).and_then(|_| Ok(())))
    }

    pub fn stop_plugin(&mut self, view_id: ViewId, name: &str) -> ClientResult<()> {
        let params = json!({"view_id": view_id, "plugin_name": name});
        Box::new(self.notify("stop", params).and_then(|_| Ok(())))
    }

    pub fn notify_plugin(
        &mut self,
        view_id: ViewId,
        plugin: &str,
        method: &str,
        params: &Value,
    ) -> ClientResult<()> {
        let params = json!({
            "view_id": view_id,
            "receiver": plugin,
            "notification": {
                "method": method,
                "params": params,
            }
        });
        Box::new(self.notify("plugin_rpc", params).and_then(|_| Ok(())))
    }

    // TODO: requests for plugin_rpc
}
