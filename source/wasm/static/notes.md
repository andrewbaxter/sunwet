design language

V1

all
    //.cont_title + el
    .cont_body_wide + templ
    //.cont_body_narrow + templ
    //.leaf_title + el

    .leaf_icon + .leaf_menu_button + el
    #menu

    .leaf_mode_button

    input
    textbox
    select
    
    title
    menu icon button
    cont section
    select
    input
    textbox

menu
    //.leaf_menu_group
    //.cont_menu_group
    //.leaf_menu_link
    //.cont_bar + .cont_bar__menu
    //.leaf_bar_button + .leaf_bar_button__menu

    group
    link
    cont buttons_menu
    in-menu text(/icon) button

query list page
    .cont_bar + .cont_bar__nonmenu
    .leaf_bar_button + .leaf_bar_button__transport

    params text/icon button
    transport icon button
    cont buttons_transport

    list element style

form
    //-

all edit pages
    //.leaf_button_free

    cont buttons_edit
    edit iconbar text(/icon) button (save, reset)
    edit free icon button (remove, add, revert, move up, move down)

node+node edit page
    //.icon_big .icon_big__incoming
    //.icon_big .icon_big__outgoing

    //center node bg is darker (menu?) + full width
    //big icon color = center node bg color

    incoming bigicon
    outgoing bigicon

node page
    edit text/icon button
    triple icon button

node edit page
    view text/icon button

V2

list edit page
    numbered list

history page
    commit title (date, or "staged")