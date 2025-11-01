test:
	./scripts/tests.sh

clean:
	cd l1 ; make clean ; 
	cd l2 ; make clean ; 

.PHONY: tests clean
