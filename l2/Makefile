PL_CLASS := L2
DST_PL_CLASS := L1
COMPILER := ../target/debug/$(PL_CLASS)
EXT_CLASS := $(PL_CLASS)
CC_CLASS := $(PL_CLASS)c

test: $(COMPILER)
	../scripts/test.sh $(EXT_CLASS) $(CC_CLASS) "tests"

test_liveness: $(COMPILER)
	./scripts/testLiveness.sh

test_interference: $(COMPILER)
	./scripts/testInterference.sh

clean:
	rm -fr *.out *.o `find tests -iname *.tmp`
	rm -fr *.$(DST_PL_CLASS)

.PHONY: test clean
