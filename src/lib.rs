#![feature(async_await)]
use futures::future::{self, join_all, BoxFuture};

#[derive(Debug)]
pub struct AdoptionError;

pub trait AdoptablePet
where
    Self: Sized,
{
    /// The id of the pet to adopt.
    type Id;

    /// Adopt the pet.
    fn do_adoption(id: &Self::Id) -> BoxFuture<'static, Result<Self, AdoptionError>>;
}

pub trait Dog: AdoptablePet
where
    // XXX: Are these all needed?
    Self: Sized + Send,
    <Self as AdoptablePet>::Id: Sync,
    Self: 'static,
    Self::AdoptingPerson: Sync,
{
    /// The Person adopting a dog.
    type AdoptingPerson;

    /// The policy to check when a person is adopting a particular dog.
    /// These would be the result of calling `can_adopt` on `AdoptionRule` above.
    fn adoption_policy(
        adopter: &Self::AdoptingPerson,
        id: &Self::Id,
    ) -> Vec<BoxFuture<'static, Result<(), AdoptionError>>>;

    /// Policy-aware adoption.
    fn adopt(
        adopter: &Self::AdoptingPerson,
        id: &Self::Id,
    ) -> BoxFuture<'static, Result<Self, AdoptionError>> {
        // [EXAMPLE A]
        // Doing the following works...
        /*
        if true {
            Self::do_adoption(id)
        } else {
            Box::pin(future::ready(Err(AdoptionError{})))
        }
        */

        /* [EXAMPLE B]
           But this is what I want to do. This is the error:

            --> src/lib.rs:71:13
             |
          71 | /             join_all(
          72 | |
          73 | |
          74 | |                 --> src/lib.rs:65:13
          ...  |
          86 | |                 Self::adoption_policy(adopter, id).iter(),
          87 | |             )
             | |_____________^ the trait `core::future::future::Future` is not implemented for `&std::pin::Pin<std::boxed::Box<dyn core::future::future::Future<Output = std::result::Result<(), AdoptionError>> + std::marker::Send>>`
             |
             = help: the following implementations were found:
                       <std::pin::Pin<P> as core::future::future::Future>
             = note: required by `futures_util::future::join_all::JoinAll`
        */
        Box::pin(
            // Check all the adoption rules in the policy.
            join_all(Self::adoption_policy(adopter, id).iter()).then(|policy_results| {
                // Depending on the result, do the (async/long-running)
                // adoption or return an error.
                let has_policy_failure = policy_results.any(|x| x.is_err());
                if !has_policy_failure {
                    Self::do_adoption(id)
                } else {
                    Box::pin(future::ready(Err(AdoptionError {})))
                }
            }),
        )
    }
}

/// Implementation.

#[derive(Debug, Clone, PartialEq)]
pub struct DogId(pub String);

pub struct Pitbull {
    pub id: DogId,
}

impl AdoptablePet for Pitbull {
    type Id = DogId;

    fn do_adoption(id: &Self::Id) -> BoxFuture<'static, Result<Self, AdoptionError>> {
        Box::pin(future::ready(Ok(Pitbull { id: id.clone() })))
    }
}

impl Dog for Pitbull {
    type AdoptingPerson = Person;
    fn adoption_policy(
        _adopter: &Self::AdoptingPerson,
        _id: &Self::Id,
    ) -> Vec<BoxFuture<'static, Result<(), AdoptionError>>> {
        vec![
            // 1. Check if they have had their shots.
            // 2. Check if the adopter has children and if the breed is good with children.
            // etc.
        ]
    }
}

pub struct Person {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        futures::executor::block_on(async {
            let id = DogId("fluffy123".to_string());
            let adopter = Person {
                name: "Fred".to_string(),
            };
            let _ = Pitbull::adopt(&adopter, &id).await.unwrap();
        });
    }
}
