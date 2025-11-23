use crate::utils::Address;

#[allow(unused)]
pub fn get_num_app() -> usize {
    unsafe extern "C" {
        safe fn _num_app();
    }
    Address::<usize>::new(_num_app as usize).read()
}
