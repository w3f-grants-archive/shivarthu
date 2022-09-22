use crate::*;

impl<T: Config> Pallet<T> {
	/// Commit your score vote
	pub(super) fn commit_vote_for_score_helper(
		key: SumTreeName,
		who: AccountIdOf<T>,
		vote_commit: [u8; 32],
	) -> DispatchResult {
		match <PeriodName<T>>::get(&key) {
			Some(period) => {
				ensure!(period == Period::Commit, Error::<T>::PeriodDontMatch);
			},
			None => Err(Error::<T>::PeriodDoesNotExists)?,
		}
		let drawn_jurors = <DrawnJurors<T>>::get(&key);
		match drawn_jurors.binary_search_by(|(c, _)| c.cmp(&who.clone())) {
			Ok(_) => {
				let vote_commit_struct = ScoreCommitVote {
					commit: vote_commit,
					votestatus: VoteStatus::Commited,
					revealed_vote: None,
				};
				<ScoreVoteCommits<T>>::insert(&key, &who, vote_commit_struct);
			},
			Err(_) => Err(Error::<T>::JurorDoesNotExists)?,
		}
		Ok(())
	}

	/// choice is i64. Validate the range of the choice while using the function
	pub(super) fn reveal_vote_score_helper(
		key: SumTreeName,
		who: AccountIdOf<T>,
		choice: i64,
		salt: Vec<u8>,
	) -> DispatchResult {
		match <PeriodName<T>>::get(&key) {
			Some(period) => {
				ensure!(period == Period::Vote, Error::<T>::PeriodDontMatch);
			},
			None => Err(Error::<T>::PeriodDoesNotExists)?,
		}
		let who_commit_vote = <ScoreVoteCommits<T>>::get(&key, &who);
		match who_commit_vote {
			Some(mut commit_struct) => {
				ensure!(
					commit_struct.votestatus == VoteStatus::Commited,
					Error::<T>::VoteStatusNotCommited
				);
				let mut vote = format!("{}", choice).as_bytes().to_vec();
				// let mut vote = choice.clone();
				let mut salt_a = salt.clone();
				vote.append(&mut salt_a);
				let vote_bytes: &[u8] = &vote;
				let hash = sp_io::hashing::keccak_256(vote_bytes);
				let commit: &[u8] = &commit_struct.commit;
				if hash == commit {
					let mut reveal_score_values = <RevealScoreValues<T>>::get(&key);
					reveal_score_values.push(choice * 1000);
					<RevealScoreValues<T>>::insert(&key, reveal_score_values);
					commit_struct.revealed_vote = Some(choice);
					commit_struct.votestatus = VoteStatus::Revealed;
					<ScoreVoteCommits<T>>::insert(&key, &who, commit_struct);
				} else {
					Err(Error::<T>::CommitDoesNotMatch)?
				}
			},
			None => Err(Error::<T>::CommitDoesNotExists)?,
		}

		Ok(())
	}

	/// Distribute incentives to juror in execution period in score schelling game
	/// Improvements: Will it be better to distribute all jurors incentives in single call
	pub(super) fn get_incentives_score_schelling_helper(
		key: SumTreeName,
		range_point: RangePoint,
	) -> DispatchResult {
		match <PeriodName<T>>::get(&key) {
			Some(period) => {
				ensure!(period == Period::Execution, Error::<T>::PeriodDontMatch);
			},
			None => Err(Error::<T>::PeriodDoesNotExists)?,
		}

		let drawn_jurors = <DrawnJurors<T>>::get(&key);
		let reveal_votes_iterator = <ScoreVoteCommits<T>>::iter_prefix(&key);
		let reveal_values = <RevealScoreValues<T>>::get(&key);
		let sd_and_mean = Self::std_deviation_interger(&reveal_values);
		let new_mean = Self::calculate_new_mean(&reveal_values, sd_and_mean).unwrap();
		<IncentiveMeanRevealScore<T>>::insert(key.clone(), new_mean);
		let incentives_range = Self::get_incentives_range(range_point);
		let reveal_votes = reveal_votes_iterator
			.map(|(account_id, score_commit_vote)| (account_id, score_commit_vote.revealed_vote))
			.collect::<Vec<(_, _)>>();
		for juror in drawn_jurors {
			match reveal_votes.binary_search_by(|(c,_)| c.cmp(&juror.0)) {
				Ok(index) => {
					let account_n_vote = &reveal_votes[index];
					if let Some(i) = account_n_vote.1 {
						if i >= new_mean.checked_sub(incentives_range).unwrap() && i <= new_mean.checked_add(incentives_range).unwrap() {
							// get incentives
						} else {
							// deduct incentives
						}
					}
				},
				Err(_) => todo!(),
			}
		}

		Ok(())
	}

	/// Calculate the mean of integer
	pub(super) fn mean_integer(data: &Vec<i64>) -> Option<i64> {
		let data_mul_sum = data.iter().sum::<i64>();
		let count = data.len();

		match count {
			positive if positive > 0 => Some(data_mul_sum / count as i64),
			_ => None,
		}
	}

	pub(super) fn std_deviation_interger(data: &Vec<i64>) -> Option<(i64, i64)> {
		let mean = Self::mean_integer(data);
		match (mean, data.len()) {
			(Some(data_mean), count) if count > 0 => {
				let variance = data
					.iter()
					.map(|value| {
						let diff = data_mean.checked_sub(*value as i64).unwrap();
						diff * diff
					})
					.sum::<i64>() / count as i64;

				Some((variance.sqrt(), mean.unwrap()))
			},
			_ => None,
		}
	}

	pub(super) fn calculate_new_mean(
		data: &Vec<i64>,
		sd_and_mean: Option<(i64, i64)>,
	) -> Option<i64> {
		let mut new_items = vec![];
		let mean = sd_and_mean.unwrap().1;
		let sd = sd_and_mean.unwrap().0;
		for x in data {
			if *x >= mean.checked_sub(sd).unwrap() && *x <= mean.checked_add(sd).unwrap() {
				new_items.push(*x);
			}
		}
		let new_mean = Self::mean_integer(&new_items);
		new_mean
	}

	pub(super) fn get_incentives_range(range_point: RangePoint) -> i64 {
		match range_point {
			RangePoint::ZeroToTen => 1500, //3 points,  1.5 ± mean, multiply by 1000 to make it integer
			RangePoint::MinusTenToPlusTen => 3000, //6 points, 3 ± mean
			RangePoint::ZeroToFive => 750, //1.5 points, 0.75 ± mean
		}
	}
}
