use courier_proto::messages::{CourieredPackage, StampedPackage, StampRequiredPackage};
use postcard::to_allocvec;

fn main() {
  let request = StampRequiredPackage::FlagRequest;
  let package = StampedPackage {
    ctr: 0x0C,
    hmac: [0x0Du8; 32],
    stamped_payload: to_allocvec(&request).unwrap()
  };
  let couriered_package = CourieredPackage::Stamped(package);

  let serialized_package = to_allocvec(&couriered_package).unwrap();

  println!("The length of the struct is {}", serialized_package.len());

  for b in serialized_package {
    print!("0x{:02x}, ", b);
  }

  println!();
}
