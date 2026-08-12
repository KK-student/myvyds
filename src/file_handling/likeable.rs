pub trait Likeable {
    fn get_likes(&self) -> u64;
    fn set_likes(&mut self, likes: u64);
    fn get_dislikes(&self) -> u64;
    fn set_dislikes(&mut self, dislikes: u64);
    fn like(&mut self) {
        self.set_likes(self.get_likes() + 1);
    }
    fn dislike(&mut self) {
        self.set_dislikes(self.get_dislikes() + 1);
    }
}
