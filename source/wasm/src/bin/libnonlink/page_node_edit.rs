use {
    super::state::set_page,
    crate::libnonlink::node_edit::build_node_edit_contents,
    flowcontrol::ta_return,
    lunk::ProcessingContext,
    rooting::El,
    shared::interface::triple::Node,
    wasm::js::{
        el_async_,
        style_export,
    },
};

pub fn build_page_node_edit(pc: &mut ProcessingContext, edit_title: &str, nodes: &Vec<Node>) {
    set_page(pc, &format!("Edit {}", edit_title), el_async_(true, {
        let eg = pc.eg();
        let nodes = nodes.clone();
        let title = edit_title.to_string();
        async move {
            ta_return!(Vec < El >, String);
            let res = build_node_edit_contents(eg, title.clone(), nodes).await?;
            return Ok(vec![style_export::cont_page_node_edit(style_export::ContPageNodeEditArgs {
                children: res.children,
                bar_children: res.bar_children,
            }).root]);
        }
    }));
}
