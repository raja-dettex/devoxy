pub trait Serializer<T> {
    fn serialize(t: T) -> Self;
    fn deserialize(&self) -> T; 
}