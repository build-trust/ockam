package node

import (
	"strconv"

	"github.com/ockam-network/ockam"
	"github.com/pkg/errors"
)

//memory commit store stores trusted full commits for use in verification
//implements ockam.Store

type MemCommitStore struct {
	commitMap map[int64]interface{}
}

func NewMemCommitStore() ockam.CommitStore {
	commitMap := make(map[int64]interface{})

	memCommitStore := MemCommitStore{
		commitMap: commitMap,
	}

	return memCommitStore
}

func Initialize(p Peer, store ockam.CommitStore) error {

	fc, err := p.FullCommit("1")
	if err != nil {
		return errors.WithStack(err)
	}

	heightInt, err := strconv.ParseInt("1", 10, 64)
	if err != nil {
		return errors.WithStack(err)
	}

	err = store.Put(heightInt, fc)
	if err != nil {
		return errors.WithStack(err)
	}

	// var latest int64 = -1
	// err = store.Put(latest, fc)
	// if err != nil {
	// 	return errors.WithStack(err)
	// }
	err = store.StoreLastTrusted(fc)
	if err != nil {
		return errors.WithStack(err)
	}

	return nil
}

func (mcs MemCommitStore) GetLastTrusted() (interface{}, error) {

	// var x int64
	// x = 1
	// fmt.Printf("%+v\n", mcs.commitMap[x])
	var latest int64 = -1
	last, err := mcs.Get(latest)
	if err != nil {
		return nil, errors.WithStack(err)
	}
	return last, nil
}

//for now, store last trusted commit with key = -1
func (mcs MemCommitStore) StoreLastTrusted(lastTrusted interface{}) error {

	var latest int64 = -1
	// err := mcs.Put(latest, lastTrusted)
	// if err != nil {
	// 	return errors.WithStack(err)
	// }
	return mcs.Put(latest, lastTrusted)
}

func (mcs MemCommitStore) Put(key, value interface{}) error {
	//commit store wants int64, attempt to cast
	keyInt, ok := key.(int64)
	if ok == false {
		return errors.New("Cannot cast key as type int64")
	}

	mcs.commitMap[keyInt] = value

	return nil
}

func (mcs MemCommitStore) Get(key interface{}) (value interface{}, err error) {
	keyInt, ok := key.(int64)
	if ok == false {
		return nil, errors.New("Cannot cast key as type int64")
	}

	value, ok = mcs.commitMap[keyInt]
	if ok == false {
		return nil, errors.New("Value does not exist")
	}

	return value, nil
}

func (mcs MemCommitStore) Delete(key interface{}) error {
	keyInt, ok := key.(int64)
	if ok == false {
		return errors.New("Cannot cast key as type int64")
	}

	//check for value's existence
	_, ok = mcs.commitMap[keyInt]
	if ok == false {
		return errors.New("Value does not exist")
	}

	delete(mcs.commitMap, keyInt)

	return nil
}
