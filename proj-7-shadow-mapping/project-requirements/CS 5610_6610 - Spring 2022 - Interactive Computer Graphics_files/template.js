$(document).ready(function(){
	// Menu ////////////////////////////////////////////////////////
	function MenuShow(elem)
	{
		$("#u-menu-bg").show();
		if ( $("#u-menu-button").is(':hidden') ) {
			$(elem).fadeIn("fast");
		} else {
			$(elem).slideDown("fast");
		}
	}
	function MenuHide(elem)
	{
		if ( $("#u-menu-button").is(':hidden') ) {
			if( $(elem).parent().parent().data('menuID') == 0 ) $("#u-menu-bg").hide(); 
			$(elem).fadeOut("fast");
		} else {
			$(elem).slideUp("fast");
		}
	}
	var menuID = 0;
	$("#u-menu").find("ul").each(function(){
		$(this).data('menuID', menuID++);
	});

	$("#u-menu-bg").click(function(){
		$(this).hide();
		$("#u-menu").find("ul").each(function(){
			if ( $(this).data('menuID') > 0 ) MenuHide($(this));
		});
		if ( $("#u-menu-button").is(':visible') ) {
			$("#u-menu").children().filter("ul").slideUp("fast");
		}
	});

	$("#u-menu").find("ul").parent().each(function(){
		if ( $(this).parent().data("menuID") >= 0 ) {
			$(this).append('<div class="u-menu-arrow">&#9654;</div>');
		}
	});

	$("#u-menu").find("li").click(function(event){
		event.stopPropagation();
		var elem = [];
		$(this).parentsUntil("#u-menu").filter("ul").each(function(){
			elem.push($(this).data('menuID'));
		});
		$("#u-menu").find("ul").each(function(){
			if ( elem.indexOf($(this).data('menuID')) < 0 ) MenuHide($(this));
		});
		if ( $(this).children().filter("ul").is(':hidden') ) {
			MenuShow( $(this).children().filter("ul") );
		} else {
			MenuHide($(this).find("ul"));
		}
	});

	$("#u-menu-button").click(function(){
		$(this).next().find("ul").slideUp("fast");
		$(this).next().slideToggle("fast",function(){
			$("#u-menu-bg").toggle( $(this).is(':visible') );
		});
	});

	$(document).keyup(function(e) {
		if ( $("#u-menu-bg").is(":visible") ) {
			//e.stopPropagation();
			if (e.keyCode === 27) $("#u-menu-bg").trigger("click");
		}
	});

	$(window).resize(function(){
		$("#u-menu").find("ul").each( function() {
			$(this).css("display","");
		});
		$("#u-menu-bg").hide();
	});

	// Scroll Top //////////////////////////////////////////////////

	$(document).scroll(function(){
		if ( $("#u-scroll-top").is(":visible") ) {
			if ( $(document).scrollTop() < 20 ) {
				$("#u-scroll-top").stop(true,true).fadeOut("slow");
			}
		} else {
			if ( $(document).scrollTop() >= 20 ) {
				$("#u-scroll-top").stop(true,true).fadeIn("slow");
			}
		}
	});

	$("#u-scroll-top").click(function(){
		$("html, body").animate({scrollTop:0}, '500', 'swing');
	});

	// Teasers ////////////////////////////////////////////////

	var teasers = [];
	$("#u-teasers").children().filter(".u-teaser").each(function(){
		teasers.push(this);
	});
	if ( teasers.length > 1 ) {
		var switchDelay = 8000;
		teasers.reverse();
		var currentTeaser = 0;
		SwitchTeaser = function(hindex) {
			if ( currentTeaser != hindex ) {
				$("#u-teasers").children().filter(".u-teaser").each(function(){
					$(this).css("z-index", 0);
					$(this).data("h_btn").className = "u-teaser-btn";
				});
				$(teasers[currentTeaser]).css("z-index",1).stop(true,true).show();
				$(teasers[hindex]).css("z-index",2).hide().fadeIn("slow");
				$(teasers[hindex]).data("h_btn").className = "u-teaser-btn-sel";
				currentTeaser = hindex;
			}
		};
		var teaserTimeout = null;
		NextTeaser = function() {
			var hindex = currentTeaser+1;
			if ( hindex >= teasers.length ) hindex = 0;
			SwitchTeaser( hindex );
			teaserTimeout = setTimeout(NextTeaser,switchDelay);
		};
		teaserTimeout = setTimeout(NextTeaser,switchDelay);
		$("#u-teasers").append('<div id="u-teaser-btns-container"><div id="u-teaser-btns"><span id="u-teaser-btn-group"></span></div></div>');
		var p = document.createElement("a");
		p.className = "u-teaser-btn-pause";
		$(p).click(function(){
			if ( teaserTimeout ) {
				clearTimeout(teaserTimeout);
				teaserTimeout = null;
				this.className = "u-teaser-btn-play";
			} else {
				teaserTimeout = setTimeout(NextTeaser,0);
				this.className = "u-teaser-btn-pause";
			}
		});
		$("#u-teaser-btn-group").append(p);
		var i = 0;
		teasers.forEach(function(){
			var b = document.createElement("a");
			b.className = i==0 ? "u-teaser-btn-sel" : "u-teaser-btn";
			$(teasers[i]).data("h_btn",b);
			$(b).data("hindex",i++).click(function(){
				var hindex = $(this).data("hindex");
				SwitchTeaser(hindex);
				if ( teaserTimeout ) {
					clearTimeout(teaserTimeout);
					teaserTimeout = setTimeout(NextTeaser,switchDelay);
				}
			});
			$("#u-teaser-btn-group").append(b);
		});
		$("#u-teasers").find(".u-teaser-info").each(function(){
			$(this).css("padding-right",($("#u-teaser-btn-group").width()+20)+"px");
		});
	}

	// Smooth Scrolling ////////////////////////////////////////////

	$('a[href^="#"]:not([href="#"])').click(function() {
	if (location.pathname.replace(/^\//,'') == this.pathname.replace(/^\//,'') && location.hostname == this.hostname) {
		var target = $(this.hash);
		target = target.length ? target : $('[name=' + this.hash.slice(1) +']');
		if (target.length) {
			$('html, body').animate({
				scrollTop: target.offset().top
			}, 500);
			return false;
		}
	}
	});

	////////////////////////////////////////////////////////////////

});