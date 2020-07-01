use com::{co_class, interfaces::IUnknown, ComRc};

use winapi::shared::minwindef::DWORD;

use crate::{
    hresult::HRESULT,
    interfaces::{
        IApplicationView, IID_IVirtualDesktopNotification, IVirtualDesktop,
        IVirtualDesktopNotification, IVirtualDesktopNotificationService,
    },
    DesktopID, HWND,
};
use std::{
    cell::{Cell, RefCell},
    ptr,
};

#[co_class(implements(IVirtualDesktopNotification))]
pub struct VirtualDesktopChangeListener {
    service: Cell<Option<ComRc<dyn IVirtualDesktopNotificationService>>>,
    cookie: Cell<u32>,
    _on_desktop_change: RefCell<Option<Box<dyn Fn(DesktopID, DesktopID) -> ()>>>,
    _on_desktop_created: RefCell<Option<Box<dyn Fn(DesktopID) -> ()>>>,
    _on_desktop_destroyed: RefCell<Option<Box<dyn Fn(DesktopID) -> ()>>>,
    _on_window_change: RefCell<Option<Box<dyn Fn(HWND) -> ()>>>,
}

impl VirtualDesktopChangeListener {
    pub fn on_desktop_change(&self, callback: Box<dyn Fn(DesktopID, DesktopID) -> ()>) {
        self._on_desktop_change.replace(Some(callback));
    }
    pub fn on_desktop_created(&self, callback: Box<dyn Fn(DesktopID) -> ()>) {
        self._on_desktop_created.replace(Some(callback));
    }
    pub fn on_desktop_destroyed(&self, callback: Box<dyn Fn(DesktopID) -> ()>) {
        self._on_desktop_destroyed.replace(Some(callback));
    }
    pub fn on_window_change(&self, callback: Box<dyn Fn(HWND) -> ()>) {
        self._on_window_change.replace(Some(callback));
    }
}

impl IVirtualDesktopNotification for VirtualDesktopChangeListener {
    unsafe fn virtual_desktop_created(&self, desktop: ComRc<dyn IVirtualDesktop>) -> HRESULT {
        if let Some(cb) = self._on_desktop_created.borrow().as_deref() {
            let mut id: DesktopID = Default::default();
            desktop.get_id(&mut id);
            cb(id);
        }
        HRESULT::ok()
    }
    unsafe fn virtual_desktop_destroy_begin(
        &self,
        _destroyed_desktop: ComRc<dyn IVirtualDesktop>,
        _fallback_desktop: ComRc<dyn IVirtualDesktop>,
    ) -> HRESULT {
        HRESULT::ok()
    }
    unsafe fn virtual_desktop_destroy_failed(
        &self,
        _destroyed_desktop: ComRc<dyn IVirtualDesktop>,
        _fallback_desktop: ComRc<dyn IVirtualDesktop>,
    ) -> HRESULT {
        HRESULT::ok()
    }
    unsafe fn virtual_desktop_destroyed(
        &self,
        destroyed_desktop: ComRc<dyn IVirtualDesktop>,
        _fallback_desktop: ComRc<dyn IVirtualDesktop>,
    ) -> HRESULT {
        if let Some(cb) = self._on_desktop_destroyed.borrow().as_deref() {
            let mut id: DesktopID = Default::default();
            destroyed_desktop.get_id(&mut id);
            cb(id);
        }
        HRESULT::ok()
    }
    unsafe fn view_virtual_desktop_changed(&self, view: ComRc<dyn IApplicationView>) -> HRESULT {
        if let Some(cb) = self._on_window_change.borrow().as_deref() {
            let mut hwnd = 0 as _;
            view.get_thumbnail_window(&mut hwnd);
            cb(hwnd as HWND);
        }
        HRESULT::ok()
    }
    unsafe fn current_virtual_desktop_changed(
        &self,
        old_desktop: ComRc<dyn IVirtualDesktop>,
        new_desktop: ComRc<dyn IVirtualDesktop>,
    ) -> HRESULT {
        if let Some(cb) = self._on_desktop_change.borrow().as_deref() {
            let mut old_id: DesktopID = Default::default();
            let mut new_id: DesktopID = Default::default();
            old_desktop.get_id(&mut old_id);
            new_desktop.get_id(&mut new_id);
            cb(old_id, new_id);
        }
        HRESULT::ok()
    }
}

impl Drop for VirtualDesktopChangeListener {
    fn drop(&mut self) {
        match self.service.take() {
            Some(s) => {
                if self.cookie.get() != 0 {
                    unsafe {
                        debug_print!("Unregister a listener {:?}", self.cookie.get());
                        s.unregister(self.cookie.get());
                    }
                }
            }
            None => {}
        }
    }
}

impl VirtualDesktopChangeListener {
    // Try to create and register a change listener
    pub(crate) fn register(
        service: ComRc<dyn IVirtualDesktopNotificationService>,
    ) -> Result<Box<VirtualDesktopChangeListener>, HRESULT> {
        let listener: Box<VirtualDesktopChangeListener> = VirtualDesktopChangeListener::new();

        // Retrieve interface pointer to IVirtualDesktopNotification
        let mut ipv = ptr::null_mut();
        let res = HRESULT::from_i32(unsafe {
            listener.query_interface(&IID_IVirtualDesktopNotification, &mut ipv)
        });
        if !res.failed() && !ipv.is_null() {
            let ptr: ComRc<dyn IVirtualDesktopNotification> =
                unsafe { ComRc::from_raw(ipv as *mut *mut _) };

            // Register the IVirtualDesktopNotification to the service
            let mut cookie: DWORD = 0;
            let res2 = unsafe { service.register(ptr, &mut cookie) };
            if res2.failed() {
                Err(res)
            } else {
                debug_print!("Register a listener {:?}", cookie);
                listener.service.set(Some(service));
                listener.cookie.set(cookie);
                Ok(listener)
            }
        } else {
            Err(res)
        }
    }

    fn new() -> Box<VirtualDesktopChangeListener> {
        VirtualDesktopChangeListener::allocate(
            Cell::new(None),
            Cell::new(0),
            RefCell::new(None),
            RefCell::new(None),
            RefCell::new(None),
            RefCell::new(None),
        )
    }
}
