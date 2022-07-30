
# gLoad

## Why do all this
So after realising all of the techincal limitations of cross compilation and stuff its bstter to just use this as a remote compiler / hoster for rust projects.

## issues
  * cross can not be run concurrently... (fix this by also having a muuuuch longer cache TTL)

## TODO before releasing:
  * DONE: Write a clear project definition in README 
  * DONE: Split up `send_binary` into multiple functions...
  * DONE: maybe move the routes into a `routes.rs` file
  * DONE: rewrite the `CurrentlyCompiling` mutex to reflect the fact that two cross compilations cannot run at the same time 
  * DONE: Fix cloning each time.
  * DONE: Need to actually use the cached path for something...
  * DONE: make it actually detect which repo is downloaded in repo_to_compile
  * DONE: A arg option for "development" where gload always pulls the repo
  * actually grab the target-triple from the target 
  * make docker work
  * A more reponsive download page (shows compilation progress etc)
  * Cleanup code
  * rewrite README

## Implementing the reponsive download page
    so to have a responsive download page we want everyone (the people waiting in line for compilation INCLUDING the person who started the compilation) to see the progress of the current compilation.

### The frontends perspective:
the front end should set get requests to a route for checking the current compilations progress, this can be part of the `CompilationProgress` which is just a struct containing:
* What stage of compilation (e.g building or compiling etc)
* how far along (in percent or something)
* What target is getting compiled

knowing the current `CompilationProgress` should let all frontends waiting show the current state.

## The backends perspective:
The current compilation state can be kept in a `Extension<Arc<Mutex<T>>>`, this will let the current compiling route update the state and the status route get the progress whenever.
    
    


## Github downloading for dummies! (githubs download button)

### General description:
so normies often have problems with downloading software from github (more specifically with getting a executable), so what happens if you want to distribute code to a largely non-technical demographic?
Probably catastrophe as YOU want to host your code on github for VCS etc but the user wants a simple download button.
This COULD be fixed by hosting binaries on sourceforge for example but this has two issues in my eyes:
 1. Its kinda sus downloading a exe from most websites.
    Exes hosted on websites could be intercepted or modified (by the 3rd party host) also: downloading random exes from randomish websites is something that should be destroyed as its a great vector for malware.
 2. How does the user know *what* version they should download?
    This is a issue with the more homegrown solutions which gets solved by bigger file hosters.

This solution could let a programmer host a simple program for a non-techincal demographic easily without worrying about file hosting or other things!

### Implementation
So this should probably run in some sort of cloud or container to allow for upscaling if downloads surges.

We could implement a rust webserver thing which redirects user to newly spun up docker instances which then compile the binary according to the system specifications and sends the compiled binary to the user and then shuts down.

heroku dyno looks great for hosting

### In depth design
    The main thing runs as a webserver and for each request it *somehow* compiles the pointed to code base for that specific cpu architecture.
    This can all be made better by the fact that we can "cache" the compiled binaries and then delete them after a while depending on how many downloads they get versus how much space they take up.

 * The program basically runs a webserver pointing to a github repository
 * The server itself might run in a docker container or something
 * When the server recieves a request it checks the users archtecture and generates a target triple thing
 * the server then checks if it has any binaries matching that target triple in cache
 * if not, check if the repo is cached, else git clone the repo
 * compile the repo for the specified target triple
 * return the generated (or existing) binary

 * How do we stop users from taking a TON of memory?

## Bugs and issues:
* If there are dependencies needed to compile the project these need to be installed before running `Gload` or if running through docker these need to be handled in other ways.
* On firefox the request might timeout if the compilation takes a long time. This can be fixed by going to `about:config` and setting `network.notify.changed` to `false`.
