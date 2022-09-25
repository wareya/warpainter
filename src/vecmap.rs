// insert-only vec-based map
pub (crate) struct VecMap<K, V>
{
    keys   : Vec<K>,
    values : Vec<V>,
}
impl<K, V> VecMap<K, V>
where K : Eq
{
    pub (crate) fn new() -> Self
    {
        Self { keys : vec!(), values : vec!() }
    }
    pub (crate) fn get<Q: ?Sized>(&self, key : &Q) -> Option<&V>
    where K : core::borrow::Borrow<Q>, Q : Eq
    {
        for (i, k) in self.keys.iter().enumerate()
        {
            if k.borrow() == key
            {
                return self.values.get(i);
            }
        }
        None
    }
    pub (crate) fn values(&self) -> &[V]
    {
        self.values.get(..).unwrap()
    }
    pub (crate) fn keys(&self) -> &[K]
    {
        self.keys.get(..).unwrap()
    }
    pub (crate) fn insert(&mut self, key : K, val : V)
    {
        for (i, k) in self.keys.iter_mut().enumerate()
        {
            if *k == key
            {
                self.values[i] = val;
                return;
            }
        }
        self.keys.push(key);
        self.values.push(val);
    }
    pub (crate) fn consume(self) -> (Vec<K>, Vec<V>)
    {
        (self.keys, self.values)
    }
}

impl<'a, K, V> core::iter::IntoIterator for &'a VecMap<K, V>
{
    type Item = &'a K;
    type IntoIter = core::slice::Iter<'a, K>;

    fn into_iter(self) -> Self::IntoIter
    {
        self.keys.iter()
    }
}
impl<'a, K, V> core::iter::IntoIterator for &'a mut VecMap<K, V>
{
    type Item = &'a mut K;
    type IntoIter = core::slice::IterMut<'a, K>;

    fn into_iter(self) -> Self::IntoIter
    {
        self.keys.iter_mut()
    }
}
impl<K, V> core::iter::IntoIterator for VecMap<K, V>
{
    type Item = K;
    type IntoIter = alloc::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter
    {
        self.keys.into_iter()
    }
}