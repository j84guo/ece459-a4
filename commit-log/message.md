# Title

Refactor message passing, reduce redundant work and batch print statements.

# Summary

This pull request attempts to increase program speed by refactoring the inter-thread message passing, reducing redundant
work, and batching print statements within each student thread. Contention between threads is reduced by using separate
channels for ideas and packages, as opposed to a single channel for both. In addition, threads compute checksums locally
and combine when they are done all processing, as opposed to locking a global checksum for each update. Redundant work 
that was removed includes re-reading data files and converting checksums to/from hex encoding. For each student thread, 
the print statements that are made for each built project are batched into one large print statement when the thread is 
about to terminate.

# Technical details

## Reducing Redundant Work

Instead of idea generators and package downloaders re-reading entire data files each time they need to generate another
idea or package, this code performs the file reading one time only by the main thread on program start. The data is
loaded into vectors and shared with the relevant threads using Arc, allowing them to use indexed access to access ideas
and packages respectively.

In a similar vein, the Checksum struct now stores raw bytes as a Vec<u8> as opposed to the hex encoded bytes as a 
String. This way, hex decoding/encoding is not necessary when updating a checksum - encoding is only necessary when
we'd like to print it.

## Reducing Contention

Each thread which performs XOR's of idea and/or package hashes does so locally, accumulating a thread-local checksum. 
When each such thread terminates, it returns its local checksum to the main thread, which combines the results of each
group of threads to obtain the final checksums. For example, the checksums returned by the idea generator threads are
combined to get the idea checksum, the checksums returned by the package downloader threads are combined to get the
package checksum, etc.

The sample code has idea generator and package downloader threads producing into a single channel, from which the 
student threads consume. As a result, students sometimes have to push ideas back into the channel, leading to increased 
contention due to having a higher number of push/pop operations. Contention is reduced by using separate channels for
ideas and packages - poison pills are pushed into the idea channel to signal student threads to terminate. 

## Batching Print Statements

Each student thread needs to print information about each project it builds. Since printing to stdout is an atomic
operation by default in Rust, concurrent printing incurs some amount of locking and thread contention. To reduce such
contention, each thread concatenates all the messages it wants to print into a stirng, and prints the concatenated 
messages prior to terminating.

# Testing for correctness

The output of the improved code is compared against the output of the sample code. For the same data files and input
parameters, the same projects should be built. Furthermore, the 2 pairs of checksums output at the end should be equal, 
i.e. idea checksum equals student idea checksum and product checksum equals student product checksum.

# Testing for performance.

The first flamegraph showed significant amount of time spent sending to and receiving from Crossbeam channels. After,
refactoring the single Event channel into 2 channels, the time spent on channel communication decreased significantly.
However, the time spent on locking shared checksums was still quite noticeable. After changing the threads to perform
checksumming locally (and combining at the end), the flamegraph boxes for locking decreased noticeably.

The next longest function call durations in the flamegraph were hex encoding and decoding. Upon looking at the Checksum
struct, it was clear that some unnecessary hex conversions were being done. Refactoring the struct to reduce these
redundant conversions helped minimize the flamegraph boxes for hex functions.

Finally, the flamegraph indicated that significant time was spent locking stdout when student threads wanted to print
that a project had been built. After batching print statements in each student thread, these boxes in the flamegraph
were less wide.

On the ecetesla0 server, hyperfine with 100 minimum runs and the default lab4 arguments indicates that the initial code 
takes approximately 260ms to run on average. After the optimizations mentioned above were applied, the average time went
down to around 5ms. This represents a 52x decrease in runtime.
