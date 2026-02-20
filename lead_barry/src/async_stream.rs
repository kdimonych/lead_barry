#![allow(dead_code)]

use core::future::poll_fn;
use core::pin::Pin;
use core::task::{Context, Poll};

// Async stream trait
pub trait AsyncStream {
    type Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

pub trait StreamExt: AsyncStream {
    /// Map combinator
    /// Creates a stream that applies a function to each item of the original stream.
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let mapped_stream = stream.map(|x| x * 2);
    /// loop {
    ///     if let Some(item) = mapped_stream.next().await {
    ///     // process item
    ///     }
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
    ///   if let Some(item) = filtered_stream.next().await {
    ///   // process item
    ///  }
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

    /// Get the next item from the stream
    /// returns None if the stream is exhausted
    async fn next(&mut self) -> Option<Self::Item>
    where
        Self: Unpin,
    {
        poll_fn(|cx| Pin::new(&mut *self).poll_next(cx)).await
    }

    /// Collect all items from the stream into a collection
    ///
    /// # Example
    /// ```rust
    /// let stream = MyStream::new();
    /// let items: heapless::Vec<i32, 16> = stream.collect().await;
    /// ```
    async fn collect<C>(mut self) -> C
    where
        Self: Sized + Unpin,
        C: Default + Extend<Self::Item>,
    {
        let mut collection = C::default();
        while let Some(item) = self.next().await {
            collection.extend(core::iter::once(item));
        }
        collection
    }
}

impl<T: AsyncStream> StreamExt for T {}

// Map combinator
pub struct Map<S, F> {
    stream: S,
    f: F,
}

impl<S, F> Map<S, F> {
    fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, U> AsyncStream for Map<S, F>
where
    S: AsyncStream,
    F: FnMut(S::Item) -> U,
{
    type Item = U;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: We are not moving out of the pinned field.
        let this = unsafe { self.get_unchecked_mut() };
        // SAFETY: We are not moving out of the pinned field. The stream itself is pinned because its parent is pinned.
        let stream = unsafe { Pin::new_unchecked(&mut this.stream) };

        match stream.poll_next(cx) {
            Poll::Ready(Some(item)) => Poll::Ready(Some((this.f)(item))),
            Poll::Ready(None) => Poll::Ready(None),
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

impl<S, F> AsyncStream for Filter<S, F>
where
    S: AsyncStream,
    F: FnMut(&S::Item) -> bool,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: We are not moving out of the pinned field.
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            // SAFETY: We are not moving out of the pinned field. The stream itself is pinned because its parent is pinned.
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    if (this.f)(&item) {
                        return Poll::Ready(Some(item));
                    }
                    // Continue polling if filter doesn't match
                }
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }
    }
}

// Filter and Map combinator
pub struct FilterAndMap<S, F> {
    stream: S,
    f: F,
}

impl<S, F> FilterAndMap<S, F> {
    fn new(stream: S, f: F) -> Self {
        Self { stream, f }
    }
}

impl<S, F, U> AsyncStream for FilterAndMap<S, F>
where
    S: AsyncStream,
    F: FnMut(&S::Item) -> Option<U>,
{
    type Item = U;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        // SAFETY: We are not moving out of the pinned field.
        let this = unsafe { self.get_unchecked_mut() };

        loop {
            // SAFETY: We are not moving out of the pinned field. The stream itself is pinned because its parent is pinned.
            let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
            match stream.poll_next(cx) {
                Poll::Ready(Some(item)) => {
                    if let Some(mapped) = (this.f)(&item) {
                        return Poll::Ready(Some(mapped));
                    }
                    // Continue polling if filter doesn't match
                }
                Poll::Ready(None) => return Poll::Ready(None),
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
        Self {
            stream,
            remaining: n,
        }
    }
}

impl<S> AsyncStream for Take<S>
where
    S: AsyncStream,
{
    type Item = S::Item;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let this = unsafe { self.get_unchecked_mut() };

        if this.remaining == 0 {
            return Poll::Ready(None);
        }

        let stream = unsafe { Pin::new_unchecked(&mut this.stream) };
        match stream.poll_next(cx) {
            Poll::Ready(Some(item)) => {
                this.remaining -= 1;
                Poll::Ready(Some(item))
            }
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}
