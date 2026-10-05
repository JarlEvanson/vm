pub struct SendSyncPointer<T: Send + Sync>(pub *const T);

unsafe impl<T: Send + Sync> Send for SendSyncPointer<T> {}
unsafe impl<T: Send + Sync> Sync for SendSyncPointer<T> {}
