use crate::shared as tr;

pub trait TrVersion {
	type RoomVertexLight;
	type RoomAmbientLight;
	type RoomLight;
	type RoomExtra;
	type MeshComponent;
	type AnimationComponent;
	type ObjectTextureDetails;
	type ObjectTextureComponent;
	type EntityComponent;
	type SoundDetailsComponent;
}

pub type Room<T> = tr::Room<<T as TrVersion>::RoomVertexLight, <T as TrVersion>::RoomAmbientLight, <T as TrVersion>::RoomLight, <T as TrVersion>::RoomExtra>;
pub type Animation<T> = tr::Animation<<T as TrVersion>::AnimationComponent>;
pub type Meshes<T> = tr::Meshes<<T as TrVersion>::MeshComponent>;
pub type ObjectTexture<T> = tr::ObjectTexture<<T as TrVersion>::ObjectTextureDetails, <T as TrVersion>::ObjectTextureComponent>;
pub type Entity<T> = tr::Entity<<T as TrVersion>::EntityComponent>;
pub type SoundDetails<T> = tr::SoundDetails<<T as TrVersion>::SoundDetailsComponent>;
