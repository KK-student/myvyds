///Represent an object that is likeable and dislikeable, such as a comment, a reply, or a video.
pub trait Likeable {
    ///Get the current amount of likes.
    fn get_likes(&self) -> u64;

    ///Set the current amount of likes.
    fn set_likes(&mut self, likes: u64);

    ///Get the current amount of dislikes.
    fn get_dislikes(&self) -> u64;

    ///Set the current amount of dislikes.
    fn set_dislikes(&mut self, dislikes: u64);

    ///Like.
    fn like(&mut self) {
        self.set_likes(self.get_likes() + 1);
    }

    ///Dislike.
    fn dislike(&mut self) {
        self.set_dislikes(self.get_dislikes() + 1);
    }
}
