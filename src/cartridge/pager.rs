#[derive(Copy, Clone, Debug)]
pub enum PageSize {
    OneKb = 0x400,
    FourKb = 0x1000,
    EightKb = 0x2000,
    SixteenKb = 0x4000,
}

#[derive(Copy, Clone, Debug)]
pub enum Page {
    First(PageSize),
    Last(PageSize),
    Number(usize, PageSize),
    FromEnd(usize, PageSize),
}

pub struct Pager {
    pub data: Vec<u8>,
}

impl Pager {
    pub fn new(data: Vec<u8>) -> Self {
        Pager { data }
    }

    pub fn read(&self, page: Page, offset: u16) -> u8 {
        let i = self.index(page, offset);
        self.data[i]
    }

    pub fn write(&mut self, page: Page, offset: u16, value: u8) {
        let i = self.index(page, offset);
        self.data[i] = value;
    }

    fn page_count(&self, size: PageSize) -> usize {
        if self.data.len() % (size as usize) != 0 {
            panic!("Page size must divide evenly into data length")
        }

        self.data.len() / (size as usize)
    }

    fn index(&self, page: Page, offset: u16) -> usize {
        match page {
            Page::First(size) => self.index(Page::Number(0, size), offset),
            Page::Last(size) => {
                let last_page = self.page_count(size) - 1;
                self.index(Page::Number(last_page, size), offset)
            }
            Page::Number(n, size) => {
                let last_page = self.page_count(size) - 1;
                if (offset as usize) > (size as usize) {
                    panic!("Offset cannot exceed page bounds")
                }
                if n > last_page {
                    panic!("Page out of bounds")
                }
                n * (size as usize) + (offset as usize)
            }
            Page::FromEnd(n, size) => {
                let last_page = self.page_count(size) - 1;
                self.index(Page::Number(last_page - n, size), offset)
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn build_pager() -> Pager {
        let mut data = Vec::new();
        for i in 0..(PageSize::SixteenKb as usize * 4) {
            data.push(i as u8);
        }
        Pager::new(data)
    }

    #[test]
    fn test_page_count() {
        let pager = build_pager();
        assert_eq!(4, pager.page_count(PageSize::SixteenKb));
        assert_eq!(8, pager.page_count(PageSize::EightKb));
        assert_eq!(16, pager.page_count(PageSize::FourKb));
    }

    #[test]
    fn test_index_first() {
        let pager = build_pager();
        assert_eq!(4, pager.index(Page::First(PageSize::SixteenKb), 4));
        assert_eq!(8, pager.index(Page::First(PageSize::SixteenKb), 8));
    }

    #[test]
    fn test_index_last() {
        let pager = build_pager();
        assert_eq!(
            0x4000 * 3 + 42,
            pager.index(Page::Last(PageSize::SixteenKb), 42)
        );
    }

    #[test]
    fn test_index_number() {
        let pager = build_pager();
        assert_eq!(
            0x1000 * 3 + 36,
            pager.index(Page::Number(3, PageSize::FourKb), 36)
        );
    }

    #[test]
    #[should_panic]
    fn test_index_overflow() {
        let pager = build_pager();
        pager.index(
            Page::First(PageSize::SixteenKb),
            PageSize::SixteenKb as u16 + 1,
        );
    }

    #[test]
    #[should_panic]
    fn test_index_nopage() {
        let pager = build_pager();
        pager.index(Page::Number(100, PageSize::SixteenKb), 0);
    }

    #[test]
    fn test_rw() {
        let mut pager = build_pager();
        pager.write(Page::Last(PageSize::FourKb), 5, 0x66);
        assert_eq!(0x66, pager.read(Page::Last(PageSize::FourKb), 5));
        assert_eq!(
            0x66,
            pager.read(Page::Last(PageSize::SixteenKb), 0x1000 * 3 + 5)
        );
    }
}
