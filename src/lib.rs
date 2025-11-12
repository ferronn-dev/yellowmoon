mod undump;

enum GCObject {
    String,
    Table,
}

enum TValue {
    GCObject(GCObject),
    Number(f64),
    Boolean(bool),
}
