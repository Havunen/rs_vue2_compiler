use lazy_static::lazy_static;
use unicase_collections::unicase_btree_set::UniCaseBTreeSet;

lazy_static! {
    static ref SVG_TAGS: UniCaseBTreeSet = {
        let mut set = UniCaseBTreeSet::new();
        let words = "svg,animate,circle,clippath,cursor,defs,desc,ellipse,filter,font-face,\
            foreignobject,g,glyph,image,line,marker,mask,missing-glyph,path,pattern,\
            polygon,polyline,rect,switch,symbol,text,textpath,tspan,use,view";
        for word in words.split(',') {
            set.insert(word.to_string());
        }
        set
    };
}

pub fn is_svg_tag(tag: &str) -> bool {
    SVG_TAGS.contains(tag)
}

pub fn get_namespace(tag: &str) -> Option<&'static str> {
    if is_svg_tag(tag) {
        Some("svg")
    } else if tag.eq_ignore_ascii_case("math") {
        Some("math")
    } else {
        None
    }
}
