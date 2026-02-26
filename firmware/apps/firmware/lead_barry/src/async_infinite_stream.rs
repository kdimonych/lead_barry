use core::future::poll_fn;
use core::pin::Pin;
use core::task::{Context, Poll};

// Async stream trait
pub trait AsyncInfiniteStream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Item>;
}

#[allow(dead_code)]
pub trait InfiniteStreamExt: AsyncInfiniteStream {
    /// Map combinator
    /// Creates a stream that applies a function to each item of the original stream.
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let mapped_stream = stream.map(|x| x * 2);
    /// loop {
    ///     let item = mapped_stream.next().await;
    ///     // process item
    /// }
    /// ```
    fn map<F, U>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> U,
    {
        Map::new(self, f)
    }

    /// Filter combinator
    /// Creates a stream that only yields items that satisfy a predicate.
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let filtered_stream = stream.filter(|x| x % 2 == 0);
    /// loop {
    ///    let item = filtered_stream.next().await;
    ///    // process item
    /// }
    /// ```
    fn filter<F>(self, f: F) -> Filter<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> bool,
    {
        Filter::new(self, f)
    }

    /// Filter and Map combinator
    /// Creates a stream that applies a function to each item of the original stream
    /// and yields only the mapped items that are Some.
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let filtered_mapped_stream = stream.map_and_filter(|x: u16| -> f32{
    ///     if x % 2 == 0u16 { Some(x * 2 as f32) } else { None }
    /// });
    /// loop {
    ///    let item = filtered_mapped_stream.next().await;
    ///    // process item
    /// }
    /// ```
    fn map_and_filter<F, U>(self, f: F) -> FilterAndMap<Self, F>
    where
        Self: Sized,
        F: FnMut(&Self::Item) -> Option<U>,
    {
        FilterAndMap::new(self, f)
    }

    /// Take combinator
    /// Creates a stream that yields only the first `n` items of the original stream.
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let limited_stream = stream.take(5);
    /// asert_eq!(limied_strem.next().await, Some(item1));
    /// asert_eq!(limied_strem.next().await, Some(item2));
    /// asert_eq!(limied_strem.next().await, Some(item3));
    /// asert_eq!(limied_strem.next().await, Some(item4));
    /// asert_eq!(limied_strem.next().await, Some(item5));
    /// asert_eq!(limied_strem.next().await, None); // No more items
    /// //Any consequent calls to next will also return None
    /// asert_eq!(limied_strem.next().await, None); // No more items
    /// ```
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take::new(self, n)
    }

    /// Collect `n` items from the stream into a collection
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let items: heapless::Vec<i32, 5> = stream.collect(5).await;
    /// ```
    async fn collect<C>(mut self, n: usize) -> C
    where
        Self: Sized + Unpin,
        C: Default + Extend<Self::Item>,
    {
        let mut collection = C::default();
        for _ in 0..n {
            let item = self.next().await;
            collection.extend(core::iter::once(item));
        }
        collection
    }

    /// Get the next item from the stream
    async fn next(&mut self) -> Self::Item
    where
        Self: Unpin,
    {
        poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }
}

impl<T: AsyncInfiniteStream> InfiniteStreamExt for T {}

// Map combinator
pub struct Map<S, F> {
    stream: S,
    f: F,
}

#[allow(dead_code)]
impl<S, F> Map<S, F> {
    fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, U> AsyncInfiniteStream for Map<S, F>
where
    S: AsyncInfiniteStream,
    F: FnMut(S::Item) -> U,
{
    type Item = U;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Item> {
        // SAFETY: We are not moving out of the pinned field.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: We are not moving out of the pinned field. The stream itself is pinned because its parent is pinned.
        let stream = unsafe { Pin::new_unchecked(&mut this.stream) };

        match stream.poll_next(cx) {
            Poll::Ready(item) => Poll::Ready((this.f)(item)),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub struct Filter<S, F> {
    stream: S,
    f: F,
}

impl<S, F> Filter<S, F> {
    fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F> AsyncInfiniteStream for Filter<S, F>
where
    S: AsyncInfiniteStream,
    F: FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Item> {
        // SAFETY: We are not moving out of the pinned field.
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            // SAFETY: We are not moving out of the pinned field. The stream itself is pinned because its parent is pinned.
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            match stream.poll_next(cx) {
                Poll::Ready(item) => {
                    if (this.f)(&item) {
                        return Poll::Ready(item);
                    }
                    // Continue polling if filter doesn't match
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

pub struct FilterAndMap<S, F> {
    stream: S,
    f: F,
}

impl<S, F> FilterAndMap<S, F> {
    fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, U> AsyncInfiniteStream for FilterAndMap<S, F>
where
    S: AsyncInfiniteStream,
    F: FnMut(&S::Item) -> Option<U>,
{
    type Item = U;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Item> {
        // SAFETY: We are not moving out of the pinned field.
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            // SAFETY: We are not moving out of the pinned field. The stream itself is pinned because its parent is pinned.
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            match stream.poll_next(cx) {
                Poll::Ready(item) => {
                    if let Some(mapped) = (this.f)(&item) {
                        return Poll::Ready(mapped);
                    }
                    // Continue polling if filter doesn't match
                }
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

// Take combinator
pub struct Take<S> {
    stream: S,
    remaining: usize,
}

impl<S> Take<S> {
    fn new(stream: S, n: usize) -> Self {
        Self { stream, remaining: n }
    }
}

impl<S> AsyncInfiniteStream for Take<S>
where
    S: AsyncInfiniteStream,
{
    type Item = Option<S::Item>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Item> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.remaining == 0 {
            return Poll::Ready(None);
        }

        let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
        match stream.poll_next(cx) {
            Poll::Ready(item) => {
                this.remaining -= 1;
                Poll::Ready(Some(item))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
