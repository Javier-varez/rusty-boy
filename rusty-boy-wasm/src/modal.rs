use yew::prelude::*;

#[derive(PartialEq, Properties)]
pub struct ModalProps {
    pub id: AttrValue,
    pub title: AttrValue,
    pub children: Html, // the field name `children` is important!
    #[prop_or_default]
    pub dismissed: Callback<MouseEvent>,
    #[prop_or_default]
    pub accepted: Option<Callback<MouseEvent>>,
}

#[function_component(Modal)]
pub fn modal(props: &ModalProps) -> Html {
    let accepted_button = props.accepted.as_ref().map(|accepted| {
        html! { <button type="button" class="btn btn-primary" onclick={accepted}>{"Accept"}</button> }
    });

    html! {
        <div class="modal" id={props.id.clone()} tabindex="-1">
          <div class="modal-dialog">
            <div class="modal-content">
              <div class="modal-header">
                <h5 class="modal-title">{props.title.clone()}</h5>
                <button type="button" class="btn-close" data-bs-dismiss="modal" aria-label="Close" onclick={props.dismissed.clone()}></button>
              </div>
              <div class="modal-body">
                { props.children.clone() }
              </div>
              <div class="modal-footer">
                <button type="button" class="btn btn-secondary" data-bs-dismiss="modal" onclick={props.dismissed.clone()}>{"Close"}</button>
                {accepted_button}
              </div>
            </div>
          </div>
        </div>
    }
}
